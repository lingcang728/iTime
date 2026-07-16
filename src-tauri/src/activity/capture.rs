use super::model::{ActivityObservation, DeviceState};
use sha2::{Digest, Sha256};
use std::path::Path;

const IDLE_THRESHOLD_MILLIS: u32 = 5 * 60 * 1_000;

fn identify(path: &Path) -> (String, String, bool) {
    let raw = path.to_string_lossy();
    let lower = raw.to_ascii_lowercase();
    let file = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Windows")
        .to_string();
    let normalized_file = file.to_ascii_lowercase();
    let (name, ai_tool) = if lower.contains("openai.codex") {
        ("Codex".to_string(), true)
    } else if ["codex", "claude", "chatgpt", "typeless"].contains(&normalized_file.as_str()) {
        let name = match normalized_file.as_str() {
            "codex" => "Codex",
            "claude" => "Claude",
            "chatgpt" => "ChatGPT",
            "typeless" => "Typeless",
            _ => file.as_str(),
        };
        (name.to_string(), true)
    } else {
        let name = match normalized_file.as_str() {
            "chrome" => "Chrome",
            "msedge" => "Microsoft Edge",
            "code" => "VS Code",
            "itime" => "iTime",
            "explorer" => "文件资源管理器",
            _ => file.as_str(),
        };
        (name.to_string(), false)
    };
    let digest = Sha256::digest(lower.as_bytes());
    (
        format!("process:{}", hex::encode(&digest[..8])),
        name,
        ai_tool,
    )
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

pub(super) fn capture_observation() -> ActivityObservation {
    let device_state = match idle_millis() {
        Some(value) if value >= IDLE_THRESHOLD_MILLIS => DeviceState::Idle,
        Some(_) => DeviceState::Active,
        None => DeviceState::Unknown,
    };
    let path = foreground_path();
    let identity = path.as_deref().map(identify);
    ActivityObservation {
        device_state,
        app_id: identity.as_ref().map(|value| value.0.clone()),
        app_name: identity.as_ref().map(|value| value.1.clone()),
        ai_tool: identity.is_some_and(|value| value.2),
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
        let (_, name, ai_tool) = identify(path);
        assert_eq!(name, "Codex");
        assert!(ai_tool);
    }

    #[test]
    fn normalizes_common_process_names_for_display() {
        assert_eq!(identify(Path::new(r"C:\Apps\chrome.exe")).1, "Chrome");
        assert_eq!(identify(Path::new(r"C:\Apps\Code.exe")).1, "VS Code");
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
