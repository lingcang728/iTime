mod activity;
mod icons;
mod keystats;
mod settings;

use activity::ActivityCollector;
use icons::IconService;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::PageLoadEvent,
    AppHandle, Emitter, LogicalSize, Manager, State, WindowEvent,
};

const DEFAULT_WINDOW_WIDTH: f64 = 1180.0;
const DEFAULT_WINDOW_HEIGHT: f64 = 760.0;
const MIN_WINDOW_WIDTH: f64 = 960.0;
const MIN_WINDOW_HEIGHT: f64 = 680.0;
const WORK_AREA_MARGIN: f64 = 16.0;

struct RuntimeState {
    recording: Arc<AtomicBool>,
    recording_generation: Arc<AtomicU64>,
    toggle_item: Mutex<Option<MenuItem<tauri::Wry>>>,
    window_fitted: AtomicBool,
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

#[tauri::command]
fn get_recording_state(state: State<'_, RuntimeState>) -> bool {
    state.recording.load(Ordering::Acquire)
}

#[tauri::command]
fn set_recording_state(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    recording: bool,
) -> Result<(), String> {
    settings::save_recording(recording)?;
    let previous = state.recording.swap(recording, Ordering::AcqRel);
    if previous != recording {
        state.recording_generation.fetch_add(1, Ordering::AcqRel);
    }
    apply_recording_state(&app, recording);
    Ok(())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let recording = Arc::new(AtomicBool::new(settings::load_recording().unwrap_or(true)));
    let recording_generation = Arc::new(AtomicU64::new(0));
    tauri::Builder::default()
        .manage(RuntimeState {
            recording: recording.clone(),
            recording_generation: recording_generation.clone(),
            toggle_item: Mutex::new(None),
            window_fitted: AtomicBool::new(false),
        })
        .manage(IconService::new())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let window = webview.window();
                let state = webview.app_handle().state::<RuntimeState>();
                if !state.window_fitted.swap(true, Ordering::AcqRel) {
                    let _ = fit_main_window_to_work_area(&window);
                }
                let _ = window.show();
                let _ = window.set_focus();
            }
        })
        .setup(|app| {
            let (recording, generation, recording_now) = {
                let runtime = app.state::<RuntimeState>();
                (
                    runtime.recording.clone(),
                    runtime.recording_generation.clone(),
                    runtime.recording.load(Ordering::Acquire),
                )
            };
            app.manage(ActivityCollector::start(recording, generation));
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
                        match settings::save_recording(recording) {
                            Ok(()) => {
                                state.recording.store(recording, Ordering::Release);
                                state.recording_generation.fetch_add(1, Ordering::AcqRel);
                                apply_recording_state(app, recording);
                            }
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
                    "quit" => app.exit(0),
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
            activity::get_activity_snapshot,
            icons::commands::resolve_app_icon,
            keystats::get_key_stats_snapshot
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
        assert_eq!(size, LogicalSize::new(1180.0, 760.0));
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
}
