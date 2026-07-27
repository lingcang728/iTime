use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
        Arc, Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::State;
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
        HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

const MINUTE_MILLIS: u64 = 60_000;
const FLUSH_INTERVAL: Duration = Duration::from_secs(3);
const CONTROL_TIMEOUT: Duration = Duration::from_secs(2);
const EVENT_QUEUE_CAPACITY: usize = 4_096;
const WRITE_ATTEMPTS: usize = 3;

const VK_SHIFT: u32 = 0x10;
const VK_CONTROL: u32 = 0x11;
const VK_MENU: u32 = 0x12;
const VK_LSHIFT: u32 = 0xa0;
const VK_RSHIFT: u32 = 0xa1;
const VK_LCONTROL: u32 = 0xa2;
const VK_RCONTROL: u32 = 0xa3;
const VK_LMENU: u32 = 0xa4;
const VK_RMENU: u32 = 0xa5;
const VK_LWIN: u32 = 0x5b;
const VK_RWIN: u32 = 0x5c;

const MOD_LCTRL: u8 = 1 << 0;
const MOD_RCTRL: u8 = 1 << 1;
const MOD_LALT: u8 = 1 << 2;
const MOD_RALT: u8 = 1 << 3;
const MOD_LWIN: u8 = 1 << 4;
const MOD_RWIN: u8 = 1 << 5;
const MOD_LSHIFT: u8 = 1 << 6;
const MOD_RSHIFT: u8 = 1 << 7;
const BLOCKING_MODIFIERS: u8 = MOD_LCTRL | MOD_RCTRL | MOD_LALT | MOD_RALT | MOD_LWIN | MOD_RWIN;

#[derive(Debug)]
enum KeyboardMessage {
    Key { timestamp: u64, generation: u64 },
    Shutdown(SyncSender<Result<(), String>>),
}

#[derive(Default)]
struct ModifierTracker {
    mask: AtomicU8,
}

impl ModifierTracker {
    fn update(&self, virtual_key: u32, pressed: bool) -> bool {
        let bits = modifier_bits(virtual_key);
        if bits == 0 {
            return false;
        }
        if pressed {
            self.mask.fetch_or(bits, Ordering::AcqRel);
        } else {
            self.mask.fetch_and(!bits, Ordering::AcqRel);
        }
        true
    }

    fn snapshot(&self) -> u8 {
        self.mask.load(Ordering::Acquire)
    }
}

struct KeyboardRuntime {
    sender: SyncSender<KeyboardMessage>,
    recording: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    modifiers: ModifierTracker,
    health: Arc<KeyboardHealth>,
}

static KEYBOARD_RUNTIME: OnceLock<KeyboardRuntime> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardRecord {
    version: u8,
    start: u64,
    #[serde(default)]
    generation: u64,
    key_strokes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyboardBucket {
    version: u8,
    start: u64,
    end: u64,
    key_strokes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardCapabilities {
    content_captured: bool,
    key_identity_captured: bool,
    direct_key_count: bool,
    modifier_combinations_excluded: bool,
    shift_character_keys_included: bool,
    granularity: &'static str,
    timezone_semantics: &'static str,
    historical_backfill: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardHealthSnapshot {
    collector_running: bool,
    writer_running: bool,
    last_write_at: Option<u64>,
    last_error: Option<String>,
    dropped_events: u64,
    write_failures: u64,
    read_failures: u64,
    queue_disconnected: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyboardSnapshot {
    source: &'static str,
    updated_at: u64,
    skipped_records: usize,
    buckets: Vec<KeyboardBucket>,
    capabilities: KeyboardCapabilities,
    health: KeyboardHealthSnapshot,
}

#[derive(Default)]
struct KeyboardHealth {
    collector_running: AtomicBool,
    writer_running: AtomicBool,
    last_write_at: AtomicU64,
    dropped_events: AtomicU64,
    write_failures: AtomicU64,
    read_failures: AtomicU64,
    queue_disconnected: AtomicBool,
    last_error: Mutex<Option<String>>,
}

impl KeyboardHealth {
    fn set_error(&self, error: impl Into<String>) {
        *self
            .last_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(error.into());
    }

    fn mark_write(&self, timestamp: u64) {
        self.last_write_at.store(timestamp, Ordering::Release);
        *self
            .last_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }

    fn snapshot(&self) -> KeyboardHealthSnapshot {
        let last_write_at = self.last_write_at.load(Ordering::Acquire);
        let queue_disconnected = self.queue_disconnected.load(Ordering::Acquire);
        let last_error = self
            .last_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .or_else(|| queue_disconnected.then(|| "键盘事件队列已断开".to_string()));
        KeyboardHealthSnapshot {
            collector_running: self.collector_running.load(Ordering::Acquire),
            writer_running: self.writer_running.load(Ordering::Acquire),
            last_write_at: (last_write_at > 0).then_some(last_write_at),
            last_error,
            dropped_events: self.dropped_events.load(Ordering::Acquire),
            write_failures: self.write_failures.load(Ordering::Acquire),
            read_failures: self.read_failures.load(Ordering::Acquire),
            queue_disconnected,
        }
    }
}

#[derive(Clone)]
pub(crate) struct KeyboardService {
    path: PathBuf,
    health: Arc<KeyboardHealth>,
}

impl KeyboardService {
    pub(crate) fn new() -> Self {
        Self {
            path: keyboard_path(),
            health: Arc::new(KeyboardHealth::default()),
        }
    }

    fn snapshot(&self, start: u64, end: u64) -> Result<KeyboardSnapshot, String> {
        read_snapshot(&self.path, start, end, &self.health)
    }
}

impl Default for KeyboardService {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct KeyboardCollector {
    sender: SyncSender<KeyboardMessage>,
    hook_thread_id: u32,
    stopped: AtomicBool,
    hook_thread: Mutex<Option<JoinHandle<()>>>,
    writer_thread: Mutex<Option<JoinHandle<()>>>,
}

impl KeyboardCollector {
    pub(crate) fn start(
        service: KeyboardService,
        recording: Arc<AtomicBool>,
        generation: Arc<AtomicU64>,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let _ = KEYBOARD_RUNTIME.set(KeyboardRuntime {
            sender: sender.clone(),
            recording,
            generation,
            modifiers: ModifierTracker::default(),
            health: service.health.clone(),
        });

        let writer_path = service.path.clone();
        let writer_health = service.health.clone();
        let writer_thread = thread::spawn(move || {
            writer_health.writer_running.store(true, Ordering::Release);
            writer_loop(receiver, &writer_path, &writer_health, FLUSH_INTERVAL);
            writer_health.writer_running.store(false, Ordering::Release);
        });

        let hook_health = service.health.clone();
        let (thread_id_sender, thread_id_receiver) = mpsc::sync_channel(1);
        let hook_thread = thread::spawn(move || unsafe {
            let thread_id = GetCurrentThreadId();
            match SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_callback),
                HINSTANCE::default(),
                0,
            ) {
                Ok(hook) => {
                    hook_health.collector_running.store(true, Ordering::Release);
                    let _ = thread_id_sender.send(thread_id);
                    let mut message = MSG::default();
                    while GetMessageW(&mut message, HWND::default(), 0, 0).as_bool() {}
                    let _ = UnhookWindowsHookEx(hook);
                    hook_health
                        .collector_running
                        .store(false, Ordering::Release);
                }
                Err(error) => {
                    hook_health.set_error(format!("无法启动 Windows 键盘计数器：{error}"));
                    let _ = thread_id_sender.send(0);
                }
            }
        });
        let hook_thread_id = thread_id_receiver
            .recv_timeout(CONTROL_TIMEOUT)
            .unwrap_or(0);

        Self {
            sender,
            hook_thread_id,
            stopped: AtomicBool::new(false),
            hook_thread: Mutex::new(Some(hook_thread)),
            writer_thread: Mutex::new(Some(writer_thread)),
        }
    }

    pub(crate) fn shutdown(&self) -> Result<(), String> {
        if self.stopped.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let shutdown_result = (|| -> Result<(), String> {
            let mut hook_thread = self
                .hook_thread
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if hook_thread.is_some() && self.hook_thread_id != 0 {
                unsafe {
                    PostThreadMessageW(
                        self.hook_thread_id,
                        WM_QUIT,
                        WPARAM::default(),
                        LPARAM::default(),
                    )
                    .map_err(|error| format!("无法停止键盘钩子：{error}"))?;
                }
            }
            if let Some(thread) = hook_thread.take() {
                thread
                    .join()
                    .map_err(|_| "键盘钩子线程退出异常".to_string())?;
            }
            drop(hook_thread);

            let (reply, response) = mpsc::sync_channel(1);
            if self.sender.send(KeyboardMessage::Shutdown(reply)).is_err() {
                let mut writer_thread = self
                    .writer_thread
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if writer_thread.as_ref().is_some_and(JoinHandle::is_finished) {
                    if let Some(thread) = writer_thread.take() {
                        thread
                            .join()
                            .map_err(|_| "键盘写入线程退出异常".to_string())?;
                    }
                    return Ok(());
                }
                return Err("键盘写入队列不可用".to_string());
            }
            response
                .recv_timeout(CONTROL_TIMEOUT)
                .map_err(|_| "键盘写入线程未及时完成退出刷新".to_string())??;
            if let Some(thread) = self
                .writer_thread
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take()
            {
                thread
                    .join()
                    .map_err(|_| "键盘写入线程退出异常".to_string())?;
            }
            Ok(())
        })();

        if shutdown_result.is_err() {
            self.stopped.store(false, Ordering::Release);
        }
        shutdown_result
    }
}

impl Drop for KeyboardCollector {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[tauri::command]
pub(crate) fn get_keyboard_snapshot(
    keyboard: State<'_, KeyboardService>,
    start: u64,
    end: u64,
) -> Result<KeyboardSnapshot, String> {
    keyboard.snapshot(start, end)
}

unsafe extern "system" fn low_level_keyboard_callback(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let message = wparam.0 as u32;
        let pressed = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
        let released = matches!(message, WM_KEYUP | WM_SYSKEYUP);
        if pressed || released {
            let key = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if let Some(runtime) = KEYBOARD_RUNTIME.get() {
                if runtime.modifiers.update(key.vkCode, pressed) {
                    return CallNextHookEx(HHOOK::default(), code, wparam, lparam);
                }
                if pressed
                    && key.flags.0 & LLKHF_INJECTED.0 == 0
                    && should_count_key(key.vkCode, runtime.modifiers.snapshot())
                    && runtime.recording.load(Ordering::Acquire)
                {
                    send_key_event(
                        runtime,
                        unix_millis(),
                        runtime.generation.load(Ordering::Acquire),
                    );
                }
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

fn modifier_bits(virtual_key: u32) -> u8 {
    match virtual_key {
        VK_CONTROL => MOD_LCTRL | MOD_RCTRL,
        VK_LCONTROL => MOD_LCTRL,
        VK_RCONTROL => MOD_RCTRL,
        VK_MENU => MOD_LALT | MOD_RALT,
        VK_LMENU => MOD_LALT,
        VK_RMENU => MOD_RALT,
        VK_LWIN => MOD_LWIN,
        VK_RWIN => MOD_RWIN,
        VK_SHIFT => MOD_LSHIFT | MOD_RSHIFT,
        VK_LSHIFT => MOD_LSHIFT,
        VK_RSHIFT => MOD_RSHIFT,
        _ => 0,
    }
}

fn should_count_key(virtual_key: u32, modifier_mask: u8) -> bool {
    is_character_key(virtual_key) && modifier_mask & BLOCKING_MODIFIERS == 0
}

fn is_character_key(virtual_key: u32) -> bool {
    matches!(
        virtual_key,
        0x20 | 0x30..=0x5a | 0x60..=0x6f | 0xba..=0xc0 | 0xdb..=0xde
    )
}

fn send_key_event(runtime: &KeyboardRuntime, timestamp: u64, generation: u64) {
    match runtime.sender.try_send(KeyboardMessage::Key {
        timestamp,
        generation,
    }) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            runtime.health.dropped_events.fetch_add(1, Ordering::AcqRel);
        }
        Err(TrySendError::Disconnected(_)) => {
            runtime
                .health
                .queue_disconnected
                .store(true, Ordering::Release);
        }
    }
}

fn writer_loop(
    receiver: Receiver<KeyboardMessage>,
    path: &Path,
    health: &KeyboardHealth,
    flush_interval: Duration,
) {
    let mut pending = BTreeMap::<(u64, u64), u64>::new();
    let mut next_flush = Instant::now() + flush_interval;
    loop {
        let wait = next_flush.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(wait) {
            Ok(KeyboardMessage::Key {
                timestamp,
                generation,
            }) => {
                let minute = timestamp / MINUTE_MILLIS * MINUTE_MILLIS;
                *pending.entry((generation, minute)).or_default() += 1;
            }
            Ok(KeyboardMessage::Shutdown(reply)) => {
                let result = flush_pending(path, &mut pending, health);
                let should_stop = result.is_ok();
                let _ = reply.send(result);
                if should_stop {
                    break;
                }
                next_flush = Instant::now() + flush_interval;
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = flush_pending(path, &mut pending, health);
                next_flush = Instant::now() + flush_interval;
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = flush_pending(path, &mut pending, health);
                break;
            }
        }
        if Instant::now() >= next_flush {
            let _ = flush_pending(path, &mut pending, health);
            next_flush = Instant::now() + flush_interval;
        }
    }
}

fn write_pending_once(path: &Path, pending: &BTreeMap<(u64, u64), u64>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    for ((generation, start), key_strokes) in pending {
        serde_json::to_writer(
            &mut writer,
            &KeyboardRecord {
                version: 1,
                start: *start,
                generation: *generation,
                key_strokes: *key_strokes,
            },
        )
        .map_err(|error| error.to_string())?;
        writer.write_all(b"\n").map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())?;
    writer
        .get_ref()
        .sync_data()
        .map_err(|error| error.to_string())
}

fn flush_pending(
    path: &Path,
    pending: &mut BTreeMap<(u64, u64), u64>,
    health: &KeyboardHealth,
) -> Result<(), String> {
    if pending.is_empty() {
        return Ok(());
    }
    let mut last_error = None;
    for attempt in 0..WRITE_ATTEMPTS {
        match write_pending_once(path, pending) {
            Ok(()) => {
                pending.clear();
                health.mark_write(unix_millis());
                return Ok(());
            }
            Err(error) => last_error = Some(error),
        }
        if attempt + 1 < WRITE_ATTEMPTS {
            thread::sleep(Duration::from_millis(50 * (attempt as u64 + 1)));
        }
    }
    health.write_failures.fetch_add(1, Ordering::AcqRel);
    let message = format!(
        "键盘字符键计数写入失败：{}",
        last_error.unwrap_or_else(|| "未知错误".into())
    );
    health.set_error(message.clone());
    Err(message)
}

fn read_snapshot(
    path: &Path,
    start: u64,
    end: u64,
    health: &KeyboardHealth,
) -> Result<KeyboardSnapshot, String> {
    if end <= start {
        return Err("键盘统计查询区间无效".into());
    }
    let mut counts = BTreeMap::<u64, u64>::new();
    let mut skipped_records = 0;
    if path.is_file() {
        let reader = BufReader::new(File::open(path).map_err(|error| error.to_string())?);
        for line in reader.lines() {
            let line = match line {
                Ok(line) => line,
                Err(error) => {
                    skipped_records += 1;
                    health.read_failures.fetch_add(1, Ordering::AcqRel);
                    health.set_error(format!("键盘字符键计数读取失败：{error}"));
                    continue;
                }
            };
            match serde_json::from_str::<KeyboardRecord>(&line) {
                Ok(record)
                    if record.version == 1
                        && record.key_strokes > 0
                        && record.start < end
                        && record.start.saturating_add(MINUTE_MILLIS) > start =>
                {
                    *counts.entry(record.start).or_default() += record.key_strokes;
                }
                Ok(_) => {}
                Err(_) => skipped_records += 1,
            }
        }
    }
    let buckets = counts
        .into_iter()
        .map(|(bucket_start, key_strokes)| KeyboardBucket {
            version: 1,
            start: bucket_start,
            end: bucket_start + MINUTE_MILLIS,
            key_strokes,
        })
        .collect();
    Ok(KeyboardSnapshot {
        source: "iTime Windows 字符键按下次数",
        updated_at: modified_millis(path).max(health.last_write_at.load(Ordering::Acquire)),
        skipped_records,
        buckets,
        capabilities: KeyboardCapabilities {
            content_captured: false,
            key_identity_captured: false,
            direct_key_count: true,
            modifier_combinations_excluded: true,
            shift_character_keys_included: true,
            granularity: "minute",
            timezone_semantics: "local-time",
            historical_backfill: false,
        },
        health: health.snapshot(),
    })
}

fn keyboard_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("iTime")
        .join("Data")
        .join("keyboard-v1.jsonl")
}

fn modified_millis(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(system_millis)
        .unwrap_or(0)
}

fn unix_millis() -> u64 {
    system_millis(SystemTime::now()).unwrap_or(0)
}

fn system_millis(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "itime-keyboard-{name}-{}-{}.jsonl",
            std::process::id(),
            unix_millis()
        ))
    }

    #[test]
    fn counts_shift_character_keys_but_excludes_ctrl_alt_and_win_combinations() {
        assert!(should_count_key(0x41, MOD_LSHIFT));
        assert!(!should_count_key(0x43, MOD_LCTRL));
        assert!(!should_count_key(0x09, MOD_LALT));
        assert!(!should_count_key(0x52, MOD_LWIN));
        assert!(!should_count_key(0x25, 0));
    }

    #[test]
    fn modifier_tracker_handles_left_and_right_keys_independently() {
        let modifiers = ModifierTracker::default();
        assert!(modifiers.update(VK_LCONTROL, true));
        assert!(modifiers.update(VK_RCONTROL, true));
        modifiers.update(VK_LCONTROL, false);
        assert_eq!(modifiers.snapshot() & MOD_RCTRL, MOD_RCTRL);
        modifiers.update(VK_RCONTROL, false);
        assert_eq!(modifiers.snapshot() & BLOCKING_MODIFIERS, 0);
    }

    #[test]
    fn queued_event_keeps_hook_time_generation_without_late_state_reinterpretation() {
        let mut pending = BTreeMap::new();
        let message = KeyboardMessage::Key {
            timestamp: MINUTE_MILLIS + 1,
            generation: 4,
        };
        if let KeyboardMessage::Key {
            timestamp,
            generation,
        } = message
        {
            let minute = timestamp / MINUTE_MILLIS * MINUTE_MILLIS;
            *pending.entry((generation, minute)).or_default() += 1;
        }
        assert_eq!(pending.get(&(4, MINUTE_MILLIS)), Some(&1));
        assert_eq!(pending.get(&(5, MINUTE_MILLIS)), None);
    }

    #[test]
    fn queue_full_is_counted_in_health() {
        let (sender, _receiver) = mpsc::sync_channel(1);
        let health = Arc::new(KeyboardHealth::default());
        let runtime = KeyboardRuntime {
            sender,
            recording: Arc::new(AtomicBool::new(true)),
            generation: Arc::new(AtomicU64::new(1)),
            modifiers: ModifierTracker::default(),
            health: health.clone(),
        };
        send_key_event(&runtime, 1, 1);
        send_key_event(&runtime, 2, 1);
        assert_eq!(health.dropped_events.load(Ordering::Acquire), 1);
    }

    #[test]
    fn continuous_input_flushes_on_independent_deadline() {
        let path = fixture_path("continuous");
        let (sender, receiver) = mpsc::sync_channel(128);
        let health = Arc::new(KeyboardHealth::default());
        let writer_health = health.clone();
        let writer_path = path.clone();
        let writer = thread::spawn(move || {
            writer_loop(
                receiver,
                &writer_path,
                &writer_health,
                Duration::from_millis(20),
            );
        });
        for index in 0..30 {
            sender
                .send(KeyboardMessage::Key {
                    timestamp: MINUTE_MILLIS + index,
                    generation: 1,
                })
                .unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            path.is_file(),
            "periodic deadline should flush before silence"
        );
        let (reply, response) = mpsc::sync_channel(1);
        sender.send(KeyboardMessage::Shutdown(reply)).unwrap();
        response.recv_timeout(CONTROL_TIMEOUT).unwrap().unwrap();
        writer.join().unwrap();
        let _ = fs::remove_file(path);
    }

    #[test]
    fn write_failure_is_bounded_reported_and_keeps_pending_data() {
        let path = fixture_path("write-failure");
        fs::create_dir_all(&path).unwrap();
        let mut pending = BTreeMap::from([((1, MINUTE_MILLIS), 3)]);
        let health = KeyboardHealth::default();
        let result = flush_pending(&path, &mut pending, &health);
        let _ = fs::remove_dir_all(&path);

        assert!(result.is_err());
        assert_eq!(pending.len(), 1);
        assert_eq!(health.write_failures.load(Ordering::Acquire), 1);
        assert!(health.snapshot().last_error.is_some());
    }

    #[test]
    fn shutdown_message_flushes_pending_events() {
        let path = fixture_path("shutdown");
        let (sender, receiver) = mpsc::sync_channel(8);
        let health = Arc::new(KeyboardHealth::default());
        let writer_health = health.clone();
        let writer_path = path.clone();
        let writer = thread::spawn(move || {
            writer_loop(
                receiver,
                &writer_path,
                &writer_health,
                Duration::from_secs(60),
            );
        });
        sender
            .send(KeyboardMessage::Key {
                timestamp: MINUTE_MILLIS,
                generation: 2,
            })
            .unwrap();
        let (reply, response) = mpsc::sync_channel(1);
        sender.send(KeyboardMessage::Shutdown(reply)).unwrap();
        response.recv_timeout(CONTROL_TIMEOUT).unwrap().unwrap();
        writer.join().unwrap();
        let snapshot = read_snapshot(&path, 0, 2 * MINUTE_MILLIS, &health).unwrap();
        let _ = fs::remove_file(path);
        assert_eq!(snapshot.buckets[0].key_strokes, 1);
    }

    #[test]
    fn bad_line_does_not_hide_later_valid_records() {
        let path = fixture_path("bad-line");
        fs::write(
            &path,
            concat!(
                r#"{"version":1,"start":60000,"generation":1,"keyStrokes":3}"#,
                "\n",
                r#"{"version":1,"start":bad}"#,
                "\n",
                r#"{"version":1,"start":120000,"generation":1,"keyStrokes":2}"#,
                "\n"
            ),
        )
        .unwrap();
        let health = KeyboardHealth::default();
        let snapshot = read_snapshot(&path, 0, 180_000, &health).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(snapshot.skipped_records, 1);
        assert_eq!(snapshot.buckets.len(), 2);
    }

    #[test]
    fn aggregates_duplicate_minute_records_without_key_identity() {
        let path = fixture_path("aggregate");
        fs::write(
            &path,
            concat!(
                r#"{"version":1,"start":60000,"keyStrokes":3}"#,
                "\n",
                r#"{"version":1,"start":60000,"keyStrokes":4}"#,
                "\n",
                r#"{"version":1,"start":120000,"keyStrokes":2}"#,
                "\n"
            ),
        )
        .unwrap();
        let health = KeyboardHealth::default();
        let snapshot = read_snapshot(&path, 0, 180_000, &health).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(snapshot.buckets.len(), 2);
        assert_eq!(snapshot.buckets[0].key_strokes, 7);
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(!json.contains("vkCode"));
        assert!(!json.contains("\"content\":"));
    }
}
