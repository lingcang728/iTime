use std::ffi::OsStr;
use std::marker::PhantomData;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use windows::core::{Interface, PCWSTR};
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

const MAX_SHORTCUT_DEPTH: u8 = 5;

#[derive(Clone, Debug)]
pub(super) struct ShortcutInfo {
    pub(super) path: PathBuf,
    pub(super) icon_path: Option<PathBuf>,
    pub(super) icon_index: i32,
}

struct ShortcutMetadata {
    target: Option<PathBuf>,
    icon_path: Option<PathBuf>,
    icon_index: i32,
}

struct ComGuard {
    should_uninitialize: bool,
    _thread_affine: PhantomData<Rc<()>>,
}

impl ComGuard {
    fn new() -> Self {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        Self {
            should_uninitialize: result.is_ok(),
            _thread_affine: PhantomData,
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe { CoUninitialize() };
        }
    }
}

pub(super) fn find_shortcut_for_executable(
    executable: &Path,
    identity: &str,
) -> Option<ShortcutInfo> {
    let expected = normalized_path(executable)?;
    let query_keys = name_keys(identity.strip_prefix("app:").unwrap_or(identity));
    for root in shortcut_roots() {
        let mut matches = Vec::new();
        collect_shortcuts(&root, MAX_SHORTCUT_DEPTH, &mut matches);
        let mut exact = matches
            .into_iter()
            .filter_map(|path| {
                let info = read_shortcut(&path)?;
                let target = shortcut_target(&path)?;
                (normalized_path(&target).as_deref() == Some(expected.as_str()))
                    .then(|| (shortcut_name_score(&path, &query_keys), info))
            })
            .collect::<Vec<_>>();
        exact.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.path.cmp(&right.1.path))
        });
        if let Some((_, info)) = exact.into_iter().next() {
            return Some(info);
        }
    }
    None
}

pub(super) fn find_shortcut_by_identity(identity: &str) -> Option<ShortcutInfo> {
    let logical = identity.strip_prefix("app:").unwrap_or(identity);
    let query_keys = name_keys(logical);
    if query_keys.is_empty() {
        return None;
    }
    for root in shortcut_roots() {
        let mut paths = Vec::new();
        collect_shortcuts(&root, MAX_SHORTCUT_DEPTH, &mut paths);
        let mut candidates = paths
            .into_iter()
            .filter_map(|path| {
                let score = shortcut_name_score(&path, &query_keys);
                (score > 0).then(|| read_shortcut(&path).map(|info| (score, info)))?
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.path.cmp(&right.1.path))
        });
        if let Some((_, info)) = candidates.into_iter().next() {
            return Some(info);
        }
    }
    None
}

fn shortcut_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(user) = std::env::var_os("USERPROFILE") {
        roots.push(PathBuf::from(user).join("Desktop"));
    }
    if let Some(public) = std::env::var_os("PUBLIC") {
        roots.push(PathBuf::from(public).join("Desktop"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let appdata = PathBuf::from(appdata);
        roots.push(appdata.join(r"Microsoft\Windows\Start Menu\Programs"));
        roots.push(appdata.join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar"));
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    roots
}

fn collect_shortcuts(directory: &Path, depth: u8, output: &mut Vec<PathBuf>) {
    if depth == 0 || !directory.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_shortcuts(&path, depth - 1, output);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        {
            output.push(path);
        }
    }
}

fn read_shortcut(path: &Path) -> Option<ShortcutInfo> {
    let metadata = load_shortcut(path)?;
    Some(ShortcutInfo {
        path: path.to_path_buf(),
        icon_path: metadata.icon_path,
        icon_index: metadata.icon_index,
    })
}

fn shortcut_target(path: &Path) -> Option<PathBuf> {
    load_shortcut(path)?.target
}

fn load_shortcut(path: &Path) -> Option<ShortcutMetadata> {
    let _com = ComGuard::new();
    let link: IShellLinkW =
        unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }.ok()?;
    let persist: IPersistFile = link.cast().ok()?;
    let wide = to_wide(path.as_os_str());
    unsafe { persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()? };

    let mut target = vec![0_u16; 32_768];
    let mut find_data = WIN32_FIND_DATAW::default();
    let target = unsafe {
        link.GetPath(&mut target, &mut find_data, SLGP_RAWPATH.0 as u32)
            .ok()
            .and_then(|()| wide_path(&target))
    };
    let mut icon = vec![0_u16; 32_768];
    let mut icon_index = 0_i32;
    let icon_path = unsafe {
        link.GetIconLocation(&mut icon, &mut icon_index)
            .ok()
            .and_then(|()| wide_path(&icon))
    };
    Some(ShortcutMetadata {
        target,
        icon_path,
        icon_index,
    })
}

fn normalized_path(path: &Path) -> Option<String> {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let value = resolved
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
        .to_ascii_lowercase();
    (!value.is_empty()).then_some(value)
}

fn wide_path(buffer: &[u16]) -> Option<PathBuf> {
    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    let value = String::from_utf16_lossy(&buffer[..length]);
    (!value.trim().is_empty()).then(|| PathBuf::from(value))
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn shortcut_name_score(path: &Path, query_keys: &[String]) -> u8 {
    let name = path
        .file_stem()
        .map(|value| value.to_string_lossy())
        .unwrap_or_default();
    match_score(query_keys, &name_keys(&name))
}

fn name_keys(value: &str) -> Vec<String> {
    let words = value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return Vec::new();
    }
    let compact = words.join("");
    let mut keys = vec![compact.clone()];
    for prefix in ["microsoft", "google", "adobe", "jetbrains"] {
        if let Some(stripped) = compact
            .strip_prefix(prefix)
            .filter(|value| value.len() >= 3)
        {
            keys.push(stripped.to_string());
        }
    }
    keys.extend(words.into_iter().filter(|word| word.chars().count() >= 4));
    keys.sort();
    keys.dedup();
    keys
}

fn match_score(query: &[String], candidate: &[String]) -> u8 {
    query
        .iter()
        .flat_map(|left| candidate.iter().map(move |right| (left, right)))
        .map(|(left, right)| {
            if left == right {
                100
            } else if left.chars().count().min(right.chars().count()) >= 4
                && (left.contains(right) || right.contains(left))
            {
                72
            } else {
                0
            }
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_registered_names_with_vendor_prefixes() {
        assert_eq!(
            match_score(&name_keys("Microsoft Word"), &name_keys("Word")),
            100
        );
        assert_eq!(
            match_score(&name_keys("JetBrains idea64"), &name_keys("idea64")),
            100
        );
    }

    #[test]
    fn rejects_unrelated_shortcut_names() {
        assert_eq!(
            match_score(&name_keys("Notion"), &name_keys("Calculator")),
            0
        );
    }
}
