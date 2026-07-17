use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSettings {
    version: u8,
    recording: bool,
}

fn settings_path() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows LOCALAPPDATA 路径不可用".to_string())?;
    Ok(PathBuf::from(local)
        .join("iTime")
        .join("Config")
        .join("settings.json"))
}

pub(crate) fn load_recording() -> Result<bool, String> {
    let path = settings_path()?;
    if !path.is_file() {
        return Ok(true);
    }
    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    let settings: RuntimeSettings =
        serde_json::from_slice(&bytes).map_err(|error| format!("记录设置损坏：{error}"))?;
    if settings.version != 1 {
        return Err("记录设置版本不受支持".into());
    }
    Ok(settings.recording)
}

pub(crate) fn save_recording(recording: bool) -> Result<(), String> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    serde_json::to_writer(
        &mut file,
        &RuntimeSettings {
            version: 1,
            recording,
        },
    )
    .map_err(|error| error.to_string())?;
    file.write_all(b"\n").map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}
