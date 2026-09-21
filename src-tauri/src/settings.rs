use crate::atomic_json;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

const SETTINGS_VERSION: u8 = 4;
pub(crate) const PROVIDER_CONSENT_VERSION: u8 = 2;
static SETTINGS_LOCK: Mutex<()> = Mutex::new(());
const CORRUPT_SETTINGS_PREFIX: &str = "settings.corrupt-";
/// A portable copy and the installed copy share the same settings.json, so
/// the in-process mutex alone cannot serialize read-modify-write between
/// them. This named mutex does (best-effort: timeout falls back to the
/// in-process lock rather than wedging every settings command).
#[cfg(windows)]
const PROCESS_LOCK_NAME: &str = r"Local\iTime-settings-write";
#[cfg(windows)]
const PROCESS_LOCK_TIMEOUT_MS: u32 = 3_000;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderConsent {
    pub(crate) version: u8,
    pub(crate) notice_seen: bool,
    #[serde(default)]
    pub(crate) ai_agent_tools_enabled: bool,
}

impl Default for ProviderConsent {
    fn default() -> Self {
        Self {
            version: PROVIDER_CONSENT_VERSION,
            notice_seen: false,
            ai_agent_tools_enabled: false,
        }
    }
}

impl ProviderConsent {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.version != PROVIDER_CONSENT_VERSION {
            return Err("AI Agent 编程工具授权版本不受支持".into());
        }
        if self.ai_agent_tools_enabled && !self.notice_seen {
            return Err("启用 AI Agent 编程工具前必须先确认本地读取说明".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSettings {
    version: u8,
    recording: bool,
    #[serde(default)]
    provider_consent: ProviderConsent,
    #[serde(default)]
    data_retention_days: Option<u16>,
}

pub(crate) fn config_dir() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows LOCALAPPDATA 路径不可用".to_string())?;
    Ok(PathBuf::from(local).join("iTime").join("Config"))
}

fn settings_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("settings.json"))
}

fn default_settings(recording: bool) -> RuntimeSettings {
    RuntimeSettings {
        version: SETTINGS_VERSION,
        recording,
        provider_consent: ProviderConsent::default(),
        data_retention_days: None,
    }
}

fn parse_settings(bytes: &[u8]) -> Result<RuntimeSettings, String> {
    let mut settings: RuntimeSettings =
        serde_json::from_slice(bytes).map_err(|error| format!("记录设置损坏：{error}"))?;
    if !matches!(settings.version, 1 | 2 | 3 | SETTINGS_VERSION) {
        return Err("记录设置版本不受支持".into());
    }
    settings.version = SETTINGS_VERSION;
    // V1 independently authorized Codex/Claude. V2 covers every registered
    // AI Agent tool plus anonymous device/performance summaries, so the old
    // choice must never silently expand into the new scope.
    if settings.provider_consent.version == 1 {
        settings.provider_consent = ProviderConsent::default();
    }
    settings.provider_consent.validate()?;
    validate_data_retention(settings.data_retention_days)?;
    Ok(settings)
}

/// 是否存在已隔离的损坏设置副本。用于在 settings.json 缺失且隔离重建写入
/// 也没成功时，仍然保持 fail-closed（不回落到 recording=true 的全新默认）。
fn has_quarantined_sibling(path: &Path) -> bool {
    let Some(dir) = path.parent() else {
        return false;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with(CORRUPT_SETTINGS_PREFIX)
    })
}

/// 把损坏/不受支持的设置文件改名隔离，并尽力写回一份 fail-closed 默认
/// （recording=false、未授权），避免坏文件把所有设置写入永久卡死。
fn quarantine_corrupt_settings(path: &Path) {
    let quarantined = path.with_file_name(format!(
        "{CORRUPT_SETTINGS_PREFIX}{}.json",
        crate::data_files::unix_millis()
    ));
    let _ = fs::rename(path, &quarantined);
    let _ = save_settings_to(path, &default_settings(false));
}

fn load_settings_from(path: &Path) -> Result<RuntimeSettings, String> {
    if !path.is_file() {
        // 全新安装默认开始记录；但已有隔离损坏副本时保持暂停（fail-closed）。
        return Ok(default_settings(!has_quarantined_sibling(path)));
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    match parse_settings(&bytes) {
        Ok(settings) => Ok(settings),
        Err(error) => {
            quarantine_corrupt_settings(path);
            eprintln!("iTime 设置文件无效，已隔离并按安全默认重建：{error}");
            Ok(default_settings(false))
        }
    }
}

fn save_settings_to(path: &Path, settings: &RuntimeSettings) -> Result<(), String> {
    atomic_json::write(path, settings)
}

/// Holds the in-process mutex plus (on Windows) the cross-process named mutex.
struct SettingsLock {
    _process: MutexGuard<'static, ()>,
    #[cfg(windows)]
    named: Option<windows::Win32::Foundation::HANDLE>,
}

impl Drop for SettingsLock {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(handle) = self.named.take() {
            // SAFETY: handle was created/acquired by acquire_named_mutex and
            // is released exactly once here.
            unsafe {
                let _ = windows::Win32::System::Threading::ReleaseMutex(handle);
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
        }
    }
}

fn settings_lock() -> SettingsLock {
    let process = SETTINGS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SettingsLock {
        _process: process,
        #[cfg(windows)]
        named: acquire_named_mutex(),
    }
}

#[cfg(windows)]
fn acquire_named_mutex() -> Option<windows::Win32::Foundation::HANDLE> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
    let name: Vec<u16> = std::ffi::OsStr::new(PROCESS_LOCK_NAME)
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: name is a valid NUL-terminated wide string; the returned handle
    // is owned by the caller and closed on failure paths below.
    unsafe {
        let handle: HANDLE = CreateMutexW(None, BOOL(0), PCWSTR(name.as_ptr())).ok()?;
        match WaitForSingleObject(handle, PROCESS_LOCK_TIMEOUT_MS) {
            // WAIT_ABANDONED still grants ownership — the previous holder died.
            WAIT_OBJECT_0 | WAIT_ABANDONED => Some(handle),
            _ => {
                let _ = CloseHandle(handle);
                None
            }
        }
    }
}

fn update_settings(
    update: impl FnOnce(&mut RuntimeSettings) -> Result<(), String>,
) -> Result<(), String> {
    let _guard = settings_lock();
    let path = settings_path()?;
    let mut settings = load_settings_from(&path)?;
    update(&mut settings)?;
    settings.version = SETTINGS_VERSION;
    save_settings_to(&path, &settings)
}

pub(crate) fn load_recording() -> Result<bool, String> {
    let _guard = settings_lock();
    Ok(load_settings_from(&settings_path()?)?.recording)
}

pub(crate) fn save_recording(recording: bool) -> Result<(), String> {
    update_settings(|settings| {
        settings.recording = recording;
        Ok(())
    })
}

pub(crate) fn sync_settings() -> Result<(), String> {
    let _guard = settings_lock();
    let path = settings_path()?;
    let settings = load_settings_from(&path)?;
    save_settings_to(&path, &settings)
}

pub(crate) fn load_provider_consent() -> Result<ProviderConsent, String> {
    let _guard = settings_lock();
    Ok(load_settings_from(&settings_path()?)?.provider_consent)
}

pub(crate) fn save_provider_consent(consent: ProviderConsent) -> Result<(), String> {
    consent.validate()?;
    update_settings(|settings| {
        settings.provider_consent = consent;
        Ok(())
    })
}

fn validate_data_retention(retention_days: Option<u16>) -> Result<(), String> {
    if matches!(retention_days, None | Some(90) | Some(365)) {
        Ok(())
    } else {
        Err("数据保留期只支持永久、365 天或 90 天".into())
    }
}

pub(crate) fn load_data_retention() -> Result<Option<u16>, String> {
    let _guard = settings_lock();
    Ok(load_settings_from(&settings_path()?)?.data_retention_days)
}

pub(crate) fn save_data_retention(retention_days: Option<u16>) -> Result<(), String> {
    validate_data_retention(retention_days)?;
    update_settings(|settings| {
        settings.data_retention_days = retention_days;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "itime-settings-{name}-{}-{}.json",
            std::process::id(),
            crate::provider_activity::unix_millis()
        ))
    }

    #[test]
    fn migrates_version_one_settings_with_provider_access_disabled() {
        let path = fixture_path("v1");
        fs::write(&path, br#"{"version":1,"recording":false}"#).unwrap();
        let settings = load_settings_from(&path).unwrap();
        let _ = fs::remove_file(path);

        assert_eq!(settings.version, SETTINGS_VERSION);
        assert!(!settings.recording);
        assert_eq!(settings.provider_consent, ProviderConsent::default());
        assert_eq!(settings.data_retention_days, None);
    }

    #[test]
    fn refuses_enabled_provider_without_notice_confirmation() {
        let consent = ProviderConsent {
            ai_agent_tools_enabled: true,
            ..ProviderConsent::default()
        };
        assert!(consent.validate().is_err());
    }

    #[test]
    fn upgrades_version_two_with_permanent_retention_by_default() {
        let path = fixture_path("v2");
        fs::write(
            &path,
            br#"{"version":2,"recording":true,"providerConsent":{"version":1,"noticeSeen":false,"codexEnabled":false,"claudeEnabled":false}}"#,
        )
        .unwrap();
        let settings = load_settings_from(&path).unwrap();
        let _ = fs::remove_file(path);
        assert_eq!(settings.version, SETTINGS_VERSION);
        assert_eq!(settings.data_retention_days, None);
        assert_eq!(settings.provider_consent, ProviderConsent::default());
    }

    #[test]
    fn resets_legacy_enabled_provider_access_for_explicit_v2_reauthorization() {
        let path = fixture_path("legacy-enabled");
        fs::write(
            &path,
            br#"{"version":3,"recording":true,"providerConsent":{"version":1,"noticeSeen":true,"codexEnabled":true,"claudeEnabled":true}}"#,
        )
        .unwrap();
        let settings = load_settings_from(&path).unwrap();
        let _ = fs::remove_file(path);

        assert_eq!(settings.provider_consent, ProviderConsent::default());
    }

    #[test]
    fn refuses_unknown_retention_period() {
        assert!(validate_data_retention(Some(30)).is_err());
        assert!(validate_data_retention(Some(90)).is_ok());
        assert!(validate_data_retention(Some(365)).is_ok());
        assert!(validate_data_retention(None).is_ok());
    }

    #[test]
    fn quarantines_corrupt_settings_and_rebuilds_fail_closed() {
        let dir = std::env::temp_dir().join(format!(
            "itime-settings-corrupt-{}-{}",
            std::process::id(),
            crate::provider_activity::unix_millis()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(&path, b"not-json{{{").unwrap();

        let settings = load_settings_from(&path).unwrap();
        assert!(!settings.recording);
        assert_eq!(settings.provider_consent, ProviderConsent::default());
        assert!(has_quarantined_sibling(&path));

        // 重建的文件立即可读且保持暂停，后续写入不再被卡死。
        let healed = load_settings_from(&path).unwrap();
        assert!(!healed.recording);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn quarantines_unsupported_future_version() {
        let dir = std::env::temp_dir().join(format!(
            "itime-settings-future-{}-{}",
            std::process::id(),
            crate::provider_activity::unix_millis()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(&path, br#"{"version":99,"recording":true}"#).unwrap();

        let settings = load_settings_from(&path).unwrap();
        assert!(!settings.recording);
        assert!(has_quarantined_sibling(&path));

        let _ = fs::remove_dir_all(dir);
    }
}
