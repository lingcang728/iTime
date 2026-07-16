use std::path::{Path, PathBuf};

pub(super) fn find_start_menu_shortcut(executable: &Path) -> Option<PathBuf> {
    let executable_name = executable
        .file_stem()?
        .to_string_lossy()
        .to_ascii_lowercase();
    let roots = [
        std::env::var_os("APPDATA")
            .map(|path| PathBuf::from(path).join(r"Microsoft\Windows\Start Menu\Programs")),
        std::env::var_os("ProgramData")
            .map(|path| PathBuf::from(path).join(r"Microsoft\Windows\Start Menu\Programs")),
    ];
    for root in roots.into_iter().flatten() {
        if let Some(found) = walk_shortcuts(&root, &executable_name, 4) {
            return Some(found);
        }
    }
    None
}

fn walk_shortcuts(directory: &Path, stem: &str, depth: u8) -> Option<PathBuf> {
    if depth == 0 || !directory.is_dir() {
        return None;
    }
    let entries = std::fs::read_dir(directory).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = walk_shortcuts(&path, stem, depth - 1) {
                return Some(found);
            }
            continue;
        }
        let is_shortcut = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"));
        if !is_shortcut {
            continue;
        }
        let name = path
            .file_stem()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if name.contains(stem) || stem.contains(&name) {
            return Some(path);
        }
    }
    None
}
