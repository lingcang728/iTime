use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

const SETTINGS_VERSION: u8 = 2;
pub(crate) const PROVIDER_CONSENT_VERSION: u8 = 1;
static SETTINGS_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderConsent {
    pub(crate) version: u8,
    pub(crate) notice_seen: bool,
    pub(crate) codex_enabled: bool,
    pub(crate) claude_enabled: bool,
}

impl Default for ProviderConsent {
    fn default() -> Self {
        Self {
            version: PROVIDER_CONSENT_VERSION,
            notice_seen: false,
            codex_enabled: false,
            claude_enabled: false,
        }
    }
}

impl ProviderConsent {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.version != PROVIDER_CONSENT_VERSION {
            return Err("Provider 授权版本不受支持".into());
        }
        if (self.codex_enabled || self.claude_enabled) && !self.notice_seen {
            return Err("启用 Provider 前必须先确认本地数据权限说明".into());
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
}

fn settings_path() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows LOCALAPPDATA 路径不可用".to_string())?;
    Ok(PathBuf::from(local)
        .join("iTime")
        .join("Config")
        .join("settings.json"))
}

fn load_settings_from(path: &Path) -> Result<RuntimeSettings, String> {
    if !path.is_file() {
        return Ok(RuntimeSettings {
            version: SETTINGS_VERSION,
            recording: true,
            provider_consent: ProviderConsent::default(),
        });
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut settings: RuntimeSettings =
        serde_json::from_slice(&bytes).map_err(|error| format!("记录设置损坏：{error}"))?;
    if !matches!(settings.version, 1 | SETTINGS_VERSION) {
        return Err("记录设置版本不受支持".into());
    }
    settings.version = SETTINGS_VERSION;
    settings.provider_consent.validate()?;
    Ok(settings)
}

fn save_settings_to(path: &Path, settings: &RuntimeSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    serde_json::to_writer(&mut file, settings).map_err(|error| error.to_string())?;
    file.write_all(b"\n").map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn update_settings(
    update: impl FnOnce(&mut RuntimeSettings) -> Result<(), String>,
) -> Result<(), String> {
    let _guard = SETTINGS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = settings_path()?;
    let mut settings = load_settings_from(&path)?;
    update(&mut settings)?;
    settings.version = SETTINGS_VERSION;
    save_settings_to(&path, &settings)
}

pub(crate) fn load_recording() -> Result<bool, String> {
    let _guard = SETTINGS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Ok(load_settings_from(&settings_path()?)?.recording)
}

pub(crate) fn save_recording(recording: bool) -> Result<(), String> {
    update_settings(|settings| {
        settings.recording = recording;
        Ok(())
    })
}

pub(crate) fn load_provider_consent() -> Result<ProviderConsent, String> {
    let _guard = SETTINGS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Ok(load_settings_from(&settings_path()?)?.provider_consent)
}

pub(crate) fn save_provider_consent(consent: ProviderConsent) -> Result<(), String> {
    consent.validate()?;
    update_settings(|settings| {
        settings.provider_consent = consent;
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
    }

    #[test]
    fn refuses_enabled_provider_without_notice_confirmation() {
        let consent = ProviderConsent {
            codex_enabled: true,
            ..ProviderConsent::default()
        };
        assert!(consent.validate().is_err());
    }
}
