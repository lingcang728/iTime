mod activity;
mod icons;
mod keyboard;
mod provider_activity;
mod reminders;
mod settings;
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
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::PageLoadEvent,
    AppHandle, Emitter, LogicalSize, Manager, State, WindowEvent,
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
    window_fitted: AtomicBool,
    maximize_on_first_show: bool,
}

fn launched_from_autostart(args: &[String]) -> bool {
    args.iter().any(|arg| arg == AUTOSTART_ARG)
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
    state: State<'_, ReminderService>,
    enabled: bool,
    interval_minutes: u64,
    quiet_start: String,
    quiet_end: String,
) -> Result<(), String> {
    state.configure(enabled, interval_minutes, &quiet_start, &quiet_end)
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
    let previous = state.recording.load(Ordering::Acquire);
    if previous == recording {
        return Ok(recording);
    }

    let previous_generation = state.recording_generation.load(Ordering::Acquire);
    let generation = previous_generation.wrapping_add(1);
    let at = unix_millis()?;
    settings::save_recording(recording)?;

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
        let rollback = settings::save_recording(previous);
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
fn set_recording_state(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    collector: State<'_, ActivityCollector>,
    recording: bool,
) -> Result<bool, String> {
    transition_recording(&app, &state, &collector, recording)
}

#[tauri::command]
fn quit_app(app: AppHandle) -> Result<(), String> {
    app.state::<ActivityCollector>().shutdown(unix_millis()?)?;
    app.state::<KeyboardCollector>().shutdown()?;
    app.exit(0);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launch_args = std::env::args().collect::<Vec<_>>();
    let recording = Arc::new(AtomicBool::new(settings::load_recording().unwrap_or(true)));
    let provider_consent = settings::load_provider_consent().unwrap_or_default();
    let recording_generation = Arc::new(AtomicU64::new(0));
    tauri::Builder::default()
        .manage(RuntimeState {
            recording: recording.clone(),
            recording_generation: recording_generation.clone(),
            recording_transition: Mutex::new(()),
            toggle_item: Mutex::new(None),
            window_fitted: AtomicBool::new(false),
            maximize_on_first_show: launched_from_autostart(&launch_args),
        })
        .manage(IconService::new())
        .manage(KeyboardService::new())
        .manage(ProviderActivityService::new(provider_consent))
        .manage(ReminderService::new())
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
                if !state.window_fitted.swap(true, Ordering::AcqRel) {
                    if state.maximize_on_first_show {
                        let _ = window.maximize();
                    } else {
                        let _ = fit_main_window_to_work_area(&window);
                    }
                }
                let _ = window.show();
                let _ = window.set_focus();
            }
        })
        .setup(|app| {
            // This identity is process-local. Persistent shell registration is
            // owned by the NSIS installer; portable builds never self-register.
            #[cfg(windows)]
            windows_shell::configure_process_identity();

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
            let reminders = MenuItem::with_id(app, "reminders", "提醒开关", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &toggle, &overview, &reminders, &quit])?;
            *app.state::<RuntimeState>()
                .toggle_item
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(toggle.clone());

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
                        let result = unix_millis()
                            .and_then(|at| app.state::<ActivityCollector>().shutdown(at))
                            .and_then(|()| app.state::<KeyboardCollector>().shutdown());
                        if let Err(error) = result {
                            let _ = app.emit("recording-error", error);
                        } else {
                            app.exit(0);
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
            quit_app,
            configure_reminders,
            activity::get_activity_snapshot,
            provider_activity::get_provider_consent,
            provider_activity::set_provider_consent,
            provider_activity::get_provider_activity_snapshot,
            icons::commands::resolve_app_icon,
            keyboard::get_keyboard_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running iTime");
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
}
