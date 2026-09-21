mod activity;
mod atomic_json;
mod data_files;
mod data_management;
mod icons;
mod keyboard;
mod provider_activity;
mod reminders;
mod settings;
mod updates;
#[cfg(windows)]
mod windows_shell;

use activity::ActivityCollector;
use icons::IconService;
use keyboard::{KeyboardCollector, KeyboardService};
use provider_activity::ProviderActivityService;
use reminders::ReminderService;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::PageLoadEvent,
    AppHandle, Emitter, LogicalSize, Manager, RunEvent, State, WindowEvent,
};

const DEFAULT_WINDOW_WIDTH: f64 = 1540.0;
const DEFAULT_WINDOW_HEIGHT: f64 = 944.0;
const MIN_WINDOW_WIDTH: f64 = 960.0;
const MIN_WINDOW_HEIGHT: f64 = 680.0;
const WORK_AREA_MARGIN: f64 = 16.0;
const AUTOSTART_ARG: &str = "--autostart";

struct RuntimeState {
    recording: Arc<AtomicBool>,
    recording_generation: Arc<AtomicU64>,
    recording_transition: Mutex<()>,
    toggle_item: Mutex<Option<MenuItem<tauri::Wry>>>,
    reminder_item: Mutex<Option<MenuItem<tauri::Wry>>>,
    window_fitted: AtomicBool,
    maximize_on_first_show: bool,
    /// 退出前落盘失败后置位：再次请求退出时不再重试，直接结束进程。
    force_exit: AtomicBool,
}

fn launched_from_autostart(args: &[String]) -> bool {
    args.iter().any(|arg| arg == AUTOSTART_ARG)
}

fn startup_recording(result: Result<bool, String>) -> bool {
    match result {
        Ok(recording) => recording,
        Err(error) => {
            eprintln!("iTime 记录设置读取失败，启动时保持暂停：{error}");
            false
        }
    }
}

fn fitted_window_size(work_width: f64, work_height: f64) -> (LogicalSize<f64>, LogicalSize<f64>) {
    let width = DEFAULT_WINDOW_WIDTH.min((work_width - WORK_AREA_MARGIN).max(1.0));
    let height = DEFAULT_WINDOW_HEIGHT.min((work_height - WORK_AREA_MARGIN).max(1.0));
    let minimum = LogicalSize::new(MIN_WINDOW_WIDTH.min(width), MIN_WINDOW_HEIGHT.min(height));
    (LogicalSize::new(width, height), minimum)
}

fn fit_main_window_to_work_area(window: &tauri::Window) -> tauri::Result<()> {
    let monitor = window.current_monitor()?.or(window.primary_monitor()?);

    if let Some(monitor) = monitor {
        let scale_factor = monitor.scale_factor();
        let work_area = monitor.work_area();
        let work_width = f64::from(work_area.size.width) / scale_factor;
        let work_height = f64::from(work_area.size.height) / scale_factor;
        let (size, minimum) = fitted_window_size(work_width, work_height);

        window.set_min_size(Some(minimum))?;
        window.set_size(size)?;
        window.center()?;
    }

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn configure_reminders(
    app: AppHandle,
    state: State<'_, ReminderService>,
    enabled: bool,
    interval_minutes: u64,
    quiet_start: String,
    quiet_end: String,
) -> Result<(), String> {
    state.configure(enabled, interval_minutes, &quiet_start, &quiet_end)?;
    // 托盘菜单项随状态切换文案，窗口藏托盘时也能看出提醒开/关。
    if let Some(item) = app
        .state::<RuntimeState>()
        .reminder_item
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
    {
        let _ = item.set_text(if enabled {
            "关闭提醒"
        } else {
            "开启提醒"
        });
    }
    Ok(())
}

/// 返回实际生效的提醒配置。前端以 localStorage 为权威副本、启动时经
/// configure_reminders 重推；重推失败时两侧会静默分叉，前端可定期
/// 用该快照对账纠偏。
#[tauri::command]
fn get_reminder_config(state: State<'_, ReminderService>) -> reminders::ReminderConfigSnapshot {
    state.config_snapshot()
}

fn apply_recording_state(app: &AppHandle, recording: bool) {
    let state = app.state::<RuntimeState>();
    if let Some(item) = state
        .toggle_item
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
    {
        let _ = item.set_text(if recording {
            "暂停记录"
        } else {
            "继续记录"
        });
    }
    if let Some(tray) = app.tray_by_id("main") {
        let label = if recording {
            "iTime · 记录中"
        } else {
            "iTime · 已暂停"
        };
        let _ = tray.set_tooltip(Some(label));
    }
    let _ = app.emit("recording-status", recording);
}

fn unix_millis() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("系统时间不可用：{error}"))
        .and_then(|duration| {
            u64::try_from(duration.as_millis()).map_err(|_| "系统时间超出支持范围".to_string())
        })
}

fn transition_recording(
    app: &AppHandle,
    state: &RuntimeState,
    collector: &ActivityCollector,
    recording: bool,
) -> Result<bool, String> {
    let _transition = state
        .recording_transition
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    transition_recording_locked(app, state, collector, recording, true)
}

fn transition_recording_locked(
    app: &AppHandle,
    state: &RuntimeState,
    collector: &ActivityCollector,
    recording: bool,
    persist_setting: bool,
) -> Result<bool, String> {
    let previous = state.recording.load(Ordering::Acquire);
    if previous == recording {
        return Ok(recording);
    }

    let previous_generation = state.recording_generation.load(Ordering::Acquire);
    let generation = previous_generation.wrapping_add(1);
    let at = unix_millis()?;
    if persist_setting {
        settings::save_recording(recording)?;
    }

    // Keyboard hook events snapshot these atomics at hook time. Update them at the same
    // command boundary that is sent to the activity collector.
    state
        .recording_generation
        .store(generation, Ordering::Release);
    state.recording.store(recording, Ordering::Release);
    if let Err(error) = collector.set_recording(recording, generation, at) {
        state.recording.store(previous, Ordering::Release);
        state
            .recording_generation
            .store(previous_generation, Ordering::Release);
        // 2s 控制超时只代表「未收到确认」——已入队的 SetRecording 仍可能被
        // worker 应用（reply 发送失败会被静默丢弃）。尽力补发一条回到原状态
        // 的命令，让 worker 与原子量重新收敛，避免记录状态双轨。
        let _ = collector.set_recording(previous, previous_generation, at);
        let rollback = if persist_setting {
            settings::save_recording(previous)
        } else {
            Ok(())
        };
        return Err(match rollback {
            Ok(()) => error,
            Err(rollback_error) => {
                format!("{error}；恢复记录设置失败：{rollback_error}")
            }
        });
    }

    apply_recording_state(app, recording);
    Ok(recording)
}

#[tauri::command]
fn get_recording_state(state: State<'_, RuntimeState>) -> bool {
    state.recording.load(Ordering::Acquire)
}

#[tauri::command]
async fn set_recording_state(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    collector: State<'_, ActivityCollector>,
    recording: bool,
) -> Result<bool, String> {
    transition_recording(&app, &state, &collector, recording)
}

fn with_local_data_quiesced<T>(
    app: &AppHandle,
    state: &RuntimeState,
    activity: &ActivityCollector,
    keyboard: &KeyboardCollector,
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let _transition = state
        .recording_transition
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let was_recording = state.recording.load(Ordering::Acquire);
    if was_recording {
        transition_recording_locked(app, state, activity, false, false)?;
    }
    let operation_result = keyboard.flush().and_then(|()| operation());
    let restore_result = if was_recording {
        transition_recording_locked(app, state, activity, true, false).map(|_| ())
    } else {
        Ok(())
    };
    match (operation_result, restore_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(restore_error)) => {
            Err(format!("数据操作完成，但恢复采集失败：{restore_error}"))
        }
        (Err(error), Err(restore_error)) => Err(format!("{error}；恢复采集失败：{restore_error}")),
    }
}

#[tauri::command]
async fn export_local_data(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    activity: State<'_, ActivityCollector>,
    keyboard: State<'_, KeyboardCollector>,
    format: String,
) -> Result<data_management::ExportResult, String> {
    with_local_data_quiesced(&app, &state, &activity, &keyboard, || {
        data_management::export(&format)
    })
}

#[tauri::command]
async fn clear_local_data(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    activity: State<'_, ActivityCollector>,
    keyboard: State<'_, KeyboardCollector>,
    providers: State<'_, ProviderActivityService>,
    confirmation: String,
) -> Result<data_management::LocalDataStatus, String> {
    if confirmation != "DELETE_ALL_LOCAL_DATA" {
        return Err("删除确认无效".into());
    }
    with_local_data_quiesced(&app, &state, &activity, &keyboard, || {
        // "删除全部"也撤销 AI Agent 工具授权：残留的授权态本身就是隐私痕迹，
        // 撤销后重新开启必须再次经过读取说明确认。
        providers.set_consent(settings::ProviderConsent::default())?;
        data_management::clear_records()
    })?;
    data_management::get_local_data_status()
}

fn request_exit(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<RuntimeState>();
    let activity = app.state::<ActivityCollector>();
    let keyboard = app.state::<KeyboardCollector>();
    let _transition = state
        .recording_transition
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let was_recording = state.recording.load(Ordering::Acquire);

    // 尽力落盘但不被写失败卡死：磁盘满/目录只读时「退出」必须仍然可用。
    // 第一次失败报错提示；用户再次退出（force_exit 已置位）则直接结束进程，
    // 退出事件里还有一轮 best-effort 收尾兜底。
    let mut flush_errors: Vec<String> = Vec::new();
    if was_recording {
        if let Err(error) = transition_recording_locked(app, &state, &activity, false, false) {
            flush_errors.push(format!("活动记录收尾失败：{error}"));
        }
    }
    if let Err(error) = keyboard.flush() {
        flush_errors.push(format!("键盘计数收尾失败：{error}"));
    }
    if flush_errors.is_empty() || state.force_exit.swap(false, Ordering::AcqRel) {
        app.exit(0);
        return Ok(());
    }
    state.force_exit.store(true, Ordering::Release);
    Err(format!(
        "{}；数据可能未完全落盘，再次退出将直接结束进程",
        flush_errors.join("；")
    ))
}

#[tauri::command]
async fn quit_app(app: AppHandle) -> Result<(), String> {
    request_exit(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launch_args = std::env::args().collect::<Vec<_>>();
    // 更新迁移重启：旧进程把 PID 传进 --relaunch-after-update=<pid>，
    // 新实例等它完全退出再初始化，否则单实例互斥会把「显示窗口」转给
    // 一个正在死亡的旧进程，更新后没有存活实例。
    #[cfg(windows)]
    if let Some(pid) = launch_args.iter().find_map(|arg| {
        arg.strip_prefix(updates::RELAUNCH_ARG_PREFIX)
            .and_then(|value| value.parse::<u32>().ok())
    }) {
        windows_shell::wait_for_process_exit(pid, Duration::from_secs(30));
    }
    let recording = Arc::new(AtomicBool::new(startup_recording(
        settings::load_recording(),
    )));
    let provider_consent = settings::load_provider_consent().unwrap_or_default();
    let recording_generation = Arc::new(AtomicU64::new(0));
    let keyboard_service = match KeyboardService::new() {
        Ok(service) => service,
        Err(error) => {
            eprintln!("iTime 本地数据目录不可用，无法安全启动：{error}");
            // windows 子系统下 stderr 不可见：至少要给用户一个原生提示。
            #[cfg(windows)]
            windows_shell::report_fatal_error(&format!(
                "iTime 无法启动：本地数据目录不可用。\n\n{error}\n\n请确认 %LOCALAPPDATA% 存在且可写后重试。"
            ));
            return;
        }
    };
    let app = tauri::Builder::default()
        .manage(RuntimeState {
            recording: recording.clone(),
            recording_generation: recording_generation.clone(),
            recording_transition: Mutex::new(()),
            toggle_item: Mutex::new(None),
            reminder_item: Mutex::new(None),
            window_fitted: AtomicBool::new(false),
            maximize_on_first_show: launched_from_autostart(&launch_args),
            force_exit: AtomicBool::new(false),
        })
        .manage(IconService::new())
        .manage(keyboard_service)
        .manage(ProviderActivityService::new(provider_consent))
        .manage(ReminderService::new())
        .manage(updates::UpdatePreparationState::default())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let window = webview.window();
                let state = webview.app_handle().state::<RuntimeState>();
                // 只在首次加载完成时拟合/显示/聚焦：之后的 webview 重载
                // （崩溃恢复等）不再抢前台焦点。
                if state.window_fitted.swap(true, Ordering::AcqRel) {
                    return;
                }
                if state.maximize_on_first_show {
                    // 自启动：显示但不 set_focus，避免登录后被抢输入焦点。
                    let _ = window.maximize();
                    let _ = window.show();
                } else {
                    let _ = fit_main_window_to_work_area(&window);
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .setup(|app| {
            // This identity is process-local. Persistent shell registration is
            // owned by the NSIS installer; portable builds never self-register.
            #[cfg(windows)]
            windows_shell::configure_process_identity();
            // 清掉上次崩溃/强杀残留的 *.tmp-<pid> 半成品（settings、
            // update-preparation 的原子写临时文件）。
            if let Ok(config_dir) = settings::config_dir() {
                atomic_json::cleanup_stale_temp(&config_dir);
            }
            if let Err(error) = data_management::apply_saved_retention() {
                eprintln!("iTime 数据保留期清理失败：{error}");
            }

            let (recording, generation, recording_now) = {
                let runtime = app.state::<RuntimeState>();
                (
                    runtime.recording.clone(),
                    runtime.recording_generation.clone(),
                    runtime.recording.load(Ordering::Acquire),
                )
            };
            let icons = (*app.state::<IconService>()).clone();
            let keyboard = (*app.state::<KeyboardService>()).clone();
            let reminders = (*app.state::<ReminderService>()).clone();
            app.manage(ActivityCollector::start(
                recording_now,
                generation.load(Ordering::Acquire),
                icons,
                reminders,
                app.handle().clone(),
            ));
            app.manage(KeyboardCollector::start(keyboard, recording, generation));
            let open = MenuItem::with_id(app, "open", "打开 iTime", true, None::<&str>)?;
            let toggle = MenuItem::with_id(
                app,
                "toggle",
                if recording_now {
                    "暂停记录"
                } else {
                    "继续记录"
                },
                true,
                None::<&str>,
            )?;
            let overview = MenuItem::with_id(app, "overview", "今日概览", true, None::<&str>)?;
            // 提醒默认关闭；前端推送配置后由 configure_reminders 同步文案。
            let reminders = MenuItem::with_id(app, "reminders", "开启提醒", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &toggle, &overview, &reminders, &quit])?;
            *app.state::<RuntimeState>()
                .toggle_item
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(toggle.clone());
            *app.state::<RuntimeState>()
                .reminder_item
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(reminders.clone());

            TrayIconBuilder::with_id("main")
                .icon(
                    app.default_window_icon()
                        .expect("application icon missing")
                        .clone(),
                )
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip(if recording_now {
                    "iTime · 记录中"
                } else {
                    "iTime · 已暂停"
                })
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "toggle" => {
                        let state = app.state::<RuntimeState>();
                        let recording = !state.recording.load(Ordering::Acquire);
                        let collector = app.state::<ActivityCollector>();
                        match transition_recording(app, &state, &collector, recording) {
                            Ok(_) => {}
                            Err(error) => {
                                let _ = app.emit("recording-error", error);
                            }
                        }
                    }
                    "overview" => {
                        show_main_window(app);
                        let _ = app.emit("navigate-to", "home");
                    }
                    "reminders" => {
                        let _ = app.emit("toggle-reminders", ());
                    }
                    "quit" => {
                        if let Err(error) = request_exit(app) {
                            let _ = app.emit("recording-error", error);
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                // The QA devtools escape hatch only exists in builds that opt
                // into the `devtools` cargo feature (`open_devtools` is gated
                // behind it — compiling this without the feature would fail).
                // Release binaries compile this out entirely.
                #[cfg(feature = "devtools")]
                if std::env::var("ITIME_NATIVE_QA").ok().as_deref() == Some("1") {
                    window.open_devtools();
                }
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = app_handle.emit("native-close-requested", ());
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_recording_state,
            set_recording_state,
            data_management::get_local_data_status,
            data_management::set_data_retention,
            data_management::open_local_data_directory,
            export_local_data,
            clear_local_data,
            quit_app,
            configure_reminders,
            get_reminder_config,
            activity::get_activity_snapshot,
            provider_activity::get_provider_consent,
            provider_activity::set_provider_consent,
            provider_activity::get_provider_activity_snapshot,
            updates::prepare_for_update,
            updates::cancel_update_preparation,
            updates::launch_migrated_install,
            icons::commands::resolve_app_icon,
            keyboard::get_keyboard_snapshot
        ])
        .build(tauri::generate_context!())
        .expect("error while building iTime");
    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { .. } = event {
            // app.exit()/系统关机不跑 Drop：这里对采集线程做一轮有界的
            // best-effort 收尾，让 pending 活动/键盘计数尽量落盘
            // （内部均有 CONTROL_TIMEOUT 封顶，不会无限拖延退出）。
            if let Some(activity) = app_handle.try_state::<ActivityCollector>() {
                let _ = activity.shutdown(unix_millis().unwrap_or(0));
            }
            if let Some(keyboard) = app_handle.try_state::<KeyboardCollector>() {
                let _ = keyboard.shutdown();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_default_size_when_work_area_is_large_enough() {
        let (size, minimum) = fitted_window_size(1920.0, 1040.0);
        assert_eq!(size, LogicalSize::new(1540.0, 944.0));
        assert_eq!(minimum, LogicalSize::new(960.0, 680.0));
    }

    #[test]
    fn constrains_size_and_minimum_to_small_high_dpi_work_area() {
        let (size, minimum) = fitted_window_size(1024.0, 720.0);
        assert_eq!(size, LogicalSize::new(1008.0, 704.0));
        assert_eq!(minimum, LogicalSize::new(960.0, 680.0));

        let (smaller_size, smaller_minimum) = fitted_window_size(960.0, 600.0);
        assert_eq!(smaller_size, LogicalSize::new(944.0, 584.0));
        assert_eq!(smaller_minimum, smaller_size);
    }

    #[test]
    fn recognizes_only_the_explicit_autostart_launch_flag() {
        assert!(launched_from_autostart(&[
            "iTime.exe".into(),
            AUTOSTART_ARG.into(),
        ]));
        assert!(!launched_from_autostart(&["iTime.exe".into()]));
    }

    #[test]
    fn recording_startup_fails_closed_on_settings_error() {
        assert!(startup_recording(Ok(true)));
        assert!(!startup_recording(Ok(false)));
        assert!(!startup_recording(Err("corrupt settings".into())));
    }
}
