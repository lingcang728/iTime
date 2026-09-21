use crate::{
    activity::ActivityCollector, atomic_json, data_management, keyboard::KeyboardCollector,
    settings, transition_recording_locked, unix_millis, RuntimeState,
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{atomic::Ordering, Mutex},
    thread,
    time::Duration,
};
use tauri::{AppHandle, State};

/// 更新迁移重启参数：旧进程把自身 PID 传给新实例，新实例等它退出后再初始化，
/// 避免单实例互斥把新实例重定向给一个正在死亡的进程（两个进程一起消失）。
pub(crate) const RELAUNCH_ARG_PREFIX: &str = "--relaunch-after-update=";
/// 被动 NSIS 安装可能很慢；轮询等待上限从 15s 放宽到 60s（在后台线程跑）。
const INSTALL_APPEAR_TIMEOUT: Duration = Duration::from_secs(60);
const INSTALL_APPEAR_POLL: Duration = Duration::from_millis(500);

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

/// 安装器在 HKCU App Paths 写入的真实 exe 路径（installer-hooks.nsh 登记）。
/// 未登记（便携/手工运行）时返回 None，由调用方回退到默认目录。
#[cfg(windows)]
fn registered_install_path() -> Option<PathBuf> {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::ERROR_MORE_DATA,
            System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ},
        },
    };

    let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\iTime.exe"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut buffer = vec![0u16; 512];
    loop {
        let mut size = (buffer.len() * std::mem::size_of::<u16>()) as u32;
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR::from_raw(subkey.as_ptr()),
                PCWSTR::null(),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(std::ptr::addr_of_mut!(size)),
            )
        };
        if result.is_ok() {
            let chars = (size as usize) / 2;
            let path = String::from_utf16_lossy(&buffer[..chars]);
            let path = path.trim_end_matches('\0');
            return (!path.is_empty()).then(|| PathBuf::from(path));
        }
        if result == ERROR_MORE_DATA && buffer.len() < 32_768 {
            buffer.resize(buffer.len() * 2, 0);
            continue;
        }
        return None;
    }
}

#[cfg(not(windows))]
fn registered_install_path() -> Option<PathBuf> {
    None
}

fn local_install_path() -> Result<PathBuf, String> {
    if let Some(registered) = registered_install_path() {
        return Ok(registered);
    }
    let local = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows LOCALAPPDATA 路径不可用".to_string())?;
    Ok(PathBuf::from(local).join("iTime").join("itime.exe"))
}

/// update-preparation.json 属于数据/迁移标记，跟随设置目录而不是安装目录——
/// 自定义安装目录时也要写到同一处。
fn preparation_path() -> Result<PathBuf, String> {
    Ok(settings::config_dir()?.join("update-preparation.json"))
}

/// 规范化到可比较的路径键：canonicalize 消解软链/8.3 短名/`\\?\` 前缀差异，
/// 再统一小写做大小写不敏感比较。
fn normalized_path_key(path: &Path) -> String {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let key = canonical.to_string_lossy();
    let key = key.strip_prefix(r"\\?\").unwrap_or(&key);
    key.to_lowercase()
}

fn is_portable_executable() -> Result<bool, String> {
    let current = std::env::current_exe().map_err(|error| error.to_string())?;
    let installed = local_install_path()?;
    Ok(normalized_path_key(&current) != normalized_path_key(&installed))
}

#[tauri::command]
pub(crate) async fn prepare_for_update(
    app: AppHandle,
    update: State<'_, UpdatePreparationState>,
    runtime: State<'_, RuntimeState>,
    activity: State<'_, ActivityCollector>,
    keyboard: State<'_, KeyboardCollector>,
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
pub(crate) async fn launch_migrated_install(
    app: AppHandle,
    update: State<'_, UpdatePreparationState>,
    runtime: State<'_, RuntimeState>,
    activity: State<'_, ActivityCollector>,
    keyboard: State<'_, KeyboardCollector>,
) -> Result<(), String> {
    // Enforce prepare_for_update first: it is the only path that quiesces
    // recording and writes the migration marker. Skipping it would exit with
    // pending data unflushed and no preparation snapshot. The state is left
    // in place (not taken) so a failed launch can still be cancelled and
    // recording restored.
    if update
        .was_recording
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_none()
    {
        return Err("尚未完成更新准备，无法迁移到安装版".into());
    }
    let installed = local_install_path()?;
    // Defensive best-effort quiesce: covers any pending data produced between
    // prepare and launch. Failures do not block the update exit — the
    // ExitRequested handler retries the flush once more.
    {
        let _transition = runtime
            .recording_transition
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if runtime.recording.load(Ordering::Acquire) {
            let _ = transition_recording_locked(&app, &runtime, &activity, false, false);
        }
        let _ = keyboard.flush();
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let deadline = std::time::Instant::now() + INSTALL_APPEAR_TIMEOUT;
        loop {
            if installed.is_file() {
                Command::new(&installed)
                    .arg(format!("{RELAUNCH_ARG_PREFIX}{}", std::process::id()))
                    .spawn()
                    .map_err(|error| format!("无法启动更新后的安装版：{error}"))?;
                // The preparation snapshot has no readers after a successful
                // relaunch — remove it instead of leaving an orphan file.
                if let Ok(path) = preparation_path() {
                    let _ = fs::remove_file(path);
                }
                app.exit(0);
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err("安装版更新完成后未找到本机安装路径".into());
            }
            thread::sleep(INSTALL_APPEAR_POLL);
        }
    })
    .await
    .map_err(|error| format!("更新迁移任务异常：{error}"))?
}

#[tauri::command]
pub(crate) async fn cancel_update_preparation(
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
