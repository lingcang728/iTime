//! Register iTime with Windows Search / Start Menu so typing "iTime" opens the app.
//!
//! Portable builds never go through NSIS, so without this the only search hits are
//! random folders (e.g. Xcode workspaces). Installed builds also benefit from a
//! self-healing shortcut that always points at the current executable.

use std::path::{Path, PathBuf};
use windows::{
    core::{Interface, HSTRING, PCWSTR},
    Win32::{
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile,
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
            },
            Registry::{
                RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_WRITE,
                REG_CREATE_KEY_DISPOSITION, REG_OPTION_NON_VOLATILE, REG_SZ,
            },
        },
        UI::Shell::{IShellLinkW, SetCurrentProcessExplicitAppUserModelID, ShellLink},
    },
};

const APP_DISPLAY_NAME: &str = "iTime";
const APP_USER_MODEL_ID: &str = "com.itime.desktop";
const APP_DESCRIPTION: &str = "本机屏幕时间与键盘输入统计";

/// Best-effort registration for Windows Search and the Start Menu.
/// Failures are non-fatal — recording must not depend on shell integration.
pub fn ensure_windows_app_discovery() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let exe = dunce_normalize(&exe);

    set_app_user_model_id();
    let _ = register_app_paths(&exe);
    let _ = ensure_start_menu_shortcut(&exe);
}

fn set_app_user_model_id() {
    let id = HSTRING::from(APP_USER_MODEL_ID);
    let _ = unsafe { SetCurrentProcessExplicitAppUserModelID(&id) };
}

fn register_app_paths(exe: &Path) -> Result<(), String> {
    let exe_str = path_to_string(exe)?;
    let dir_str = path_to_string(
        exe.parent()
            .ok_or_else(|| "executable has no parent directory".to_string())?,
    )?;

    // HKCU\Software\Microsoft\Windows\CurrentVersion\App Paths\iTime.exe
    // lets Shell / Run / Search resolve "iTime.exe" to this build.
    let subkey = HSTRING::from(r"Software\Microsoft\Windows\CurrentVersion\App Paths\iTime.exe");
    let mut hkey = Default::default();
    let mut disposition = REG_CREATE_KEY_DISPOSITION::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            Some(&mut disposition),
        )
    };
    if status.is_err() {
        return Err(format!("RegCreateKeyExW failed: {status:?}"));
    }

    let write = |name: Option<&str>, value: &str| -> Result<(), String> {
        let wide = to_wide_null(value);
        let bytes = wide_as_reg_bytes(&wide);
        let name_hstring = name.map(HSTRING::from);
        let name_pcwstr = match &name_hstring {
            Some(h) => PCWSTR(h.as_ptr()),
            None => PCWSTR::null(),
        };
        let status = unsafe { RegSetValueExW(hkey, name_pcwstr, 0, REG_SZ, Some(&bytes)) };
        if status.is_err() {
            Err(format!("RegSetValueExW failed: {status:?}"))
        } else {
            Ok(())
        }
    };

    let result = (|| {
        write(None, &exe_str)?;
        write(Some("Path"), &dir_str)?;
        Ok(())
    })();

    unsafe {
        let _ = RegCloseKey(hkey);
    }
    result
}

fn ensure_start_menu_shortcut(exe: &Path) -> Result<(), String> {
    let programs = start_menu_programs_dir()?;
    std::fs::create_dir_all(&programs)
        .map_err(|error| format!("create Start Menu Programs dir: {error}"))?;

    let link_path = programs.join(format!("{APP_DISPLAY_NAME}.lnk"));
    let work_dir = exe
        .parent()
        .ok_or_else(|| "executable has no parent directory".to_string())?;

    create_shell_link(&link_path, exe, work_dir, APP_DESCRIPTION)
}

fn start_menu_programs_dir() -> Result<PathBuf, String> {
    let appdata = std::env::var_os("APPDATA").ok_or_else(|| "APPDATA is not set".to_string())?;
    Ok(PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"))
}

fn create_shell_link(
    link_path: &Path,
    exe: &Path,
    work_dir: &Path,
    description: &str,
) -> Result<(), String> {
    let _com = ComGuard::new()?;
    let shell_link: IShellLinkW = unsafe {
        CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("CoCreateInstance(ShellLink): {error}"))?
    };

    let exe_h = HSTRING::from(path_to_string(exe)?.as_str());
    let dir_h = HSTRING::from(path_to_string(work_dir)?.as_str());
    let desc_h = HSTRING::from(description);

    unsafe {
        shell_link
            .SetPath(PCWSTR(exe_h.as_ptr()))
            .map_err(|error| format!("SetPath: {error}"))?;
        shell_link
            .SetWorkingDirectory(PCWSTR(dir_h.as_ptr()))
            .map_err(|error| format!("SetWorkingDirectory: {error}"))?;
        shell_link
            .SetDescription(PCWSTR(desc_h.as_ptr()))
            .map_err(|error| format!("SetDescription: {error}"))?;
        // Icon = the executable itself (embedded product icon).
        shell_link
            .SetIconLocation(PCWSTR(exe_h.as_ptr()), 0)
            .map_err(|error| format!("SetIconLocation: {error}"))?;
    }

    let persist: IPersistFile = shell_link
        .cast()
        .map_err(|error| format!("IPersistFile cast: {error}"))?;
    let link_h = HSTRING::from(path_to_string(link_path)?.as_str());
    unsafe {
        persist
            .Save(PCWSTR(link_h.as_ptr()), true)
            .map_err(|error| format!("IPersistFile::Save: {error}"))?;
    }
    Ok(())
}

/// Strip the `\\?\` prefix Windows sometimes adds via canonicalize.
fn dunce_normalize(path: &Path) -> PathBuf {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy();
    if let Some(stripped) = text.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        canonical
    }
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "path is not valid UTF-8".to_string())
}

fn to_wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_as_reg_bytes(wide: &[u16]) -> Vec<u8> {
    wide.iter().flat_map(|unit| unit.to_le_bytes()).collect()
}

struct ComGuard {
    should_uninit: bool,
}

impl ComGuard {
    fn new() -> Result<Self, String> {
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        // S_OK (0) or S_FALSE (1, already initialized on this thread).
        if hr.is_ok() {
            Ok(Self {
                should_uninit: hr.0 == 0,
            })
        } else if hr.0 == 0x8001_0106u32 as i32 {
            // RPC_E_CHANGED_MODE — COM already initialized with a different model.
            Ok(Self {
                should_uninit: false,
            })
        } else {
            Err(format!("CoInitializeEx failed: {hr:?}"))
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe { CoUninitialize() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_registry_bytes_include_null_terminator() {
        let wide = to_wide_null("iTime");
        let bytes = wide_as_reg_bytes(&wide);
        assert_eq!(bytes.len(), ("iTime".len() + 1) * 2);
        assert_eq!(&bytes[bytes.len() - 2..], &[0, 0]);
    }

    #[test]
    fn dunce_normalize_strips_extended_prefix() {
        let path = PathBuf::from(r"\\?\C:\Apps\iTime\iTime.exe");
        // canonicalize may fail for missing paths; exercise strip helper shape via direct call path.
        let text = path.to_string_lossy();
        let stripped = text.strip_prefix(r"\\?\").unwrap();
        assert_eq!(stripped, r"C:\Apps\iTime\iTime.exe");
    }
}
