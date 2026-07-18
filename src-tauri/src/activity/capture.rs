use super::model::{ActivityObservation, DeviceState};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(super) const IDLE_THRESHOLD_MILLIS: u32 = 5 * 60 * 1_000;

pub(super) struct CapturedObservation {
    pub(super) observation: ActivityObservation,
    pub(super) idle_millis: Option<u32>,
    pub(super) icon_hint: Option<(String, PathBuf)>,
}

struct IdentifiedApp {
    app_id: String,
    app_name: String,
    icon_key: String,
    ai_tool: bool,
    system_surface: bool,
}

fn logical_key(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if separator && !output.is_empty() {
                output.push('-');
            }
            output.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    output
}

fn identify(path: &Path) -> IdentifiedApp {
    let raw = path.to_string_lossy();
    let lower = raw.to_ascii_lowercase();
    let file = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Windows")
        .to_string();
    let normalized_file = file.to_ascii_lowercase();
    let (name, icon_key, ai_tool, system_surface) = if lower.contains("microsoft.lockapp")
        || normalized_file == "lockapp"
        || normalized_file == "logonui"
    {
        ("Windows 锁屏", "windows-lock", false, true)
    } else if lower.contains("openai.codex")
        || matches!(normalized_file.as_str(), "codex" | "codex-code-mode-host")
    {
        ("Codex", "codex", true, false)
    } else if matches!(normalized_file.as_str(), "claude" | "chatgpt") {
        let (name, key) = match normalized_file.as_str() {
            "claude" => ("Claude", "claude"),
            "chatgpt" => ("ChatGPT", "chatgpt"),
            _ => (file.as_str(), normalized_file.as_str()),
        };
        (name, key, true, false)
    } else {
        let (name, key) = match normalized_file.as_str() {
            "chrome" => ("Chrome", "chrome"),
            "msedge" => ("Microsoft Edge", "msedge"),
            "code" => ("VS Code", "vscode"),
            "itime" => ("iTime", "itime"),
            "explorer" => ("文件资源管理器", "explorer"),
            "weixin" | "wechat" => ("微信", "wechat"),
            "windowsterminal" => ("Windows Terminal", "windows-terminal"),
            "clash-verge" => ("Clash Verge", "clash-verge"),
            _ => (file.as_str(), normalized_file.as_str()),
        };
        (name, key, false, false)
    };
    let digest = Sha256::digest(lower.as_bytes());
    let stable_key = logical_key(icon_key);
    let app_id = if matches!(
        stable_key.as_str(),
        "chrome"
            | "msedge"
            | "vscode"
            | "itime"
            | "explorer"
            | "wechat"
            | "windows-terminal"
            | "clash-verge"
            | "codex"
            | "claude"
            | "chatgpt"
    ) {
        format!("app:{stable_key}")
    } else {
        format!("process:{}", hex::encode(&digest[..8]))
    };
    IdentifiedApp {
        app_id,
        app_name: name.to_string(),
        icon_key: stable_key,
        ai_tool,
        system_surface,
    }
}

#[cfg(windows)]
struct OwnedProcess(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        use windows::Win32::Foundation::CloseHandle;
        // SAFETY: this wrapper is created only from a successful OpenProcess call and owns the
        // handle exclusively; Drop runs exactly once and ignores only the close status.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(windows)]
fn idle_millis() -> Option<u32> {
    use windows::Win32::{
        System::SystemInformation::GetTickCount64,
        UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
    };
    let mut input = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `input` is initialized with the documented structure size and remains writable
    // for the duration of the synchronous call. GetTickCount64 has no pointer arguments.
    let valid = unsafe { GetLastInputInfo(&mut input).as_bool() };
    if !valid {
        return None;
    }
    // SAFETY: GetTickCount64 has no preconditions and returns the current system tick count.
    let current = unsafe { GetTickCount64() } as u32;
    Some(elapsed_ticks(current, input.dwTime))
}

#[cfg(windows)]
fn foreground_path() -> Option<std::path::PathBuf> {
    use windows::{
        core::PWSTR,
        Win32::{
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
            UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    };
    // SAFETY: GetForegroundWindow returns a borrowed HWND. The PID out-pointer is valid for the
    // synchronous call, and no window handle ownership is transferred.
    let process_id = unsafe {
        let window = GetForegroundWindow();
        if window.0.is_null() {
            return None;
        }
        let mut process_id = 0;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        process_id
    };
    if process_id == 0 {
        return None;
    }
    // SAFETY: The PID came from the current foreground window. The returned owned handle is
    // closed exactly once below after the synchronous image-path query.
    let process = OwnedProcess(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?
    });
    let mut buffer = vec![0_u16; 32_768];
    let mut length = u32::try_from(buffer.len()).ok()?;
    // SAFETY: `buffer` is uniquely owned and initialized, its capacity is reported in `length`,
    // and it cannot reallocate during the synchronous call. `process` remains valid until close.
    let result = unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    result.ok()?;
    let used = usize::try_from(length).ok()?.min(buffer.len());
    Some(std::path::PathBuf::from(String::from_utf16_lossy(
        &buffer[..used],
    )))
}

#[cfg(not(windows))]
fn idle_millis() -> Option<u32> {
    None
}

#[cfg(not(windows))]
fn foreground_path() -> Option<std::path::PathBuf> {
    None
}

pub(super) fn capture_observation() -> CapturedObservation {
    let idle_millis = idle_millis();
    let mut device_state = match idle_millis {
        Some(value) if value >= IDLE_THRESHOLD_MILLIS => DeviceState::Idle,
        Some(_) => DeviceState::Active,
        None => DeviceState::Unknown,
    };
    let path = foreground_path();
    let identity = path.as_deref().map(identify);
    if identity.as_ref().is_some_and(|value| value.system_surface) {
        device_state = DeviceState::Locked;
    }
    let visible_identity = identity.as_ref().filter(|value| !value.system_surface);
    let icon_hint = visible_identity.and_then(|value| {
        path.clone()
            .map(|path| (format!("app:{}", value.icon_key), path))
    });
    CapturedObservation {
        observation: ActivityObservation {
            device_state,
            app_id: visible_identity.map(|value| value.app_id.clone()),
            app_name: visible_identity.map(|value| value.app_name.clone()),
            ai_tool: visible_identity.is_some_and(|value| value.ai_tool),
        },
        idle_millis,
        icon_hint,
    }
}

fn elapsed_ticks(current: u32, last_input: u32) -> u32 {
    current.wrapping_sub(last_input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_codex_package_without_reading_window_titles() {
        let path = Path::new(r"C:\Program Files\WindowsApps\OpenAI.Codex_1\app\ChatGPT.exe");
        let app = identify(path);
        assert_eq!(app.app_name, "Codex");
        assert_eq!(app.app_id, "app:codex");
        assert!(app.ai_tool);
    }

    #[test]
    fn normalizes_common_process_names_for_display() {
        assert_eq!(
            identify(Path::new(r"C:\Apps\chrome.exe")).app_name,
            "Chrome"
        );
        assert_eq!(identify(Path::new(r"C:\Apps\Code.exe")).app_name, "VS Code");
    }

    #[test]
    fn treats_lock_app_as_a_system_surface() {
        let app = identify(Path::new(
            r"C:\Windows\SystemApps\Microsoft.LockApp_1\LockApp.exe",
        ));
        assert!(app.system_surface);
        assert_eq!(app.icon_key, "windows-lock");
    }

    #[test]
    fn normalizes_common_apps_to_stable_product_ids() {
        assert_eq!(
            identify(Path::new(r"C:\Apps\Weixin.exe")).app_id,
            "app:wechat"
        );
        assert_eq!(
            identify(Path::new(r"C:\Apps\WindowsTerminal.exe")).app_id,
            "app:windows-terminal"
        );
        assert_eq!(
            identify(Path::new(r"G:\Clash\clash-verge.exe")).app_id,
            "app:clash-verge"
        );
    }

    #[test]
    fn calculates_idle_ticks_across_u32_wraparound() {
        assert_eq!(elapsed_ticks(20, u32::MAX - 9), 30);
    }

    #[test]
    fn serialized_observation_contains_no_path_title_or_pid() {
        let observation = ActivityObservation {
            device_state: DeviceState::Active,
            app_id: Some("process:abc".into()),
            app_name: Some("Code".into()),
            ai_tool: false,
        };
        let json = serde_json::to_string(&observation).expect("serializable observation");
        assert!(!json.to_ascii_lowercase().contains("path"));
        assert!(!json.to_ascii_lowercase().contains("title"));
        assert!(!json.contains("\"pid\""));
        assert!(!json.contains("processId"));
    }
}
