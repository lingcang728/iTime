use crate::{
    activity::ActivityCollector, atomic_json, data_management, keyboard::KeyboardCollector,
    settings, telemetry::TelemetryService, transition_recording_locked, unix_millis, RuntimeState,
};
use serde::Serialize;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::{atomic::Ordering, Mutex},
    thread,
    time::Duration,
};
use tauri::{AppHandle, State};

#[derive(Default)]
pub(crate) struct UpdatePreparationState {
    was_recording: Mutex<Option<bool>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdatePreparation {
    schema_version: u8,
    prepared_at: u64,
    previous_version: &'static str,
    was_recording: bool,
    portable: bool,
    local_data: data_management::LocalDataStatus,
}

fn local_install_path() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows LOCALAPPDATA 路径不可用".to_string())?;
    Ok(PathBuf::from(local).join("iTime").join("itime.exe"))
}

fn preparation_path() -> Result<PathBuf, String> {
    Ok(local_install_path()?
        .parent()
        .ok_or_else(|| "iTime 安装路径无效".to_string())?
        .join("Config")
        .join("update-preparation.json"))
}

fn is_portable_executable() -> Result<bool, String> {
    let current = std::env::current_exe().map_err(|error| error.to_string())?;
    let installed = local_install_path()?;
    Ok(!current
        .to_string_lossy()
        .eq_ignore_ascii_case(&installed.to_string_lossy()))
}

#[tauri::command]
pub(crate) fn prepare_for_update(
    app: AppHandle,
    update: State<'_, UpdatePreparationState>,
    runtime: State<'_, RuntimeState>,
    activity: State<'_, ActivityCollector>,
    keyboard: State<'_, KeyboardCollector>,
    telemetry: State<'_, TelemetryService>,
) -> Result<UpdatePreparation, String> {
    let mut prepared = update
        .was_recording
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if prepared.is_some() {
        return Err("更新准备已完成，不能重复执行".into());
    }

    let _transition = runtime
        .recording_transition
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let was_recording = runtime.recording.load(Ordering::Acquire);
    if was_recording {
        transition_recording_locked(&app, &runtime, &activity, false, false)?;
    }

    let operation = (|| {
        keyboard.flush()?;
        settings::sync_settings()?;
        telemetry.prepare_for_update()?;
        let preparation = UpdatePreparation {
            schema_version: 1,
            prepared_at: unix_millis()?,
            previous_version: env!("CARGO_PKG_VERSION"),
            was_recording,
            portable: is_portable_executable()?,
            local_data: data_management::get_local_data_status()?,
        };
        atomic_json::write(&preparation_path()?, &preparation)?;
        Ok(preparation)
    })();

    match operation {
        Ok(preparation) => {
            *prepared = Some(was_recording);
            Ok(preparation)
        }
        Err(error) => {
            let restore = if was_recording {
                transition_recording_locked(&app, &runtime, &activity, true, false).map(|_| ())
            } else {
                Ok(())
            };
            match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(format!("{error}；恢复采集失败：{restore_error}")),
            }
        }
    }
}

#[tauri::command]
pub(crate) fn launch_migrated_install(app: AppHandle) -> Result<(), String> {
    let installed = local_install_path()?;
    for _ in 0..30 {
        if installed.is_file() {
            Command::new(&installed)
                .spawn()
                .map_err(|error| format!("无法启动更新后的安装版：{error}"))?;
            app.exit(0);
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err("安装版更新完成后未找到本机安装路径".into())
}

#[tauri::command]
pub(crate) fn cancel_update_preparation(
    app: AppHandle,
    update: State<'_, UpdatePreparationState>,
    runtime: State<'_, RuntimeState>,
    activity: State<'_, ActivityCollector>,
) -> Result<(), String> {
    let mut prepared = update
        .was_recording
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(was_recording) = prepared.take() else {
        return Ok(());
    };
    let _transition = runtime
        .recording_transition
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if was_recording && !runtime.recording.load(Ordering::Acquire) {
        transition_recording_locked(&app, &runtime, &activity, true, false)?;
    }
    if let Ok(path) = preparation_path() {
        let _ = fs::remove_file(path);
    }
    Ok(())
}
