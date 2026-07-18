use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
        Arc, Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::State;
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
        HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT,
        WM_SYSKEYDOWN,
    },
};

const MINUTE_MILLIS: u64 = 60_000;
const FLUSH_INTERVAL: Duration = Duration::from_secs(3);
const EVENT_QUEUE_CAPACITY: usize = 4_096;

#[derive(Clone, Copy, Debug)]
enum KeyboardMessage {
    Key(u64),
    Shutdown,
}

static KEYBOARD_EVENTS: OnceLock<SyncSender<KeyboardMessage>> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardRecord {
    version: u8,
    start: u64,
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
    granularity: &'static str,
    timezone_semantics: &'static str,
    historical_backfill: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardHealthSnapshot {
    collector_running: bool,
    last_write_at: Option<u64>,
    last_error: Option<String>,
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
    last_write_at: AtomicU64,
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
        KeyboardHealthSnapshot {
            collector_running: self.collector_running.load(Ordering::Acquire),
            last_write_at: (last_write_at > 0).then_some(last_write_at),
            last_error: self
                .last_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
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
    hook_thread: Option<JoinHandle<()>>,
    writer_thread: Option<JoinHandle<()>>,
}

impl KeyboardCollector {
    pub(crate) fn start(service: KeyboardService, recording: Arc<AtomicBool>) -> Self {
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let _ = KEYBOARD_EVENTS.set(sender.clone());

        let writer_path = service.path.clone();
        let writer_health = service.health.clone();
        let writer_thread =
            thread::spawn(move || writer_loop(receiver, &writer_path, &writer_health, &recording));

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
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or(0);

        Self {
            sender,
            hook_thread_id,
            hook_thread: Some(hook_thread),
            writer_thread: Some(writer_thread),
        }
    }
}

impl Drop for KeyboardCollector {
    fn drop(&mut self) {
        let _ = self.sender.send(KeyboardMessage::Shutdown);
        if self.hook_thread_id != 0 {
            unsafe {
                let _ = PostThreadMessageW(
                    self.hook_thread_id,
                    WM_QUIT,
                    WPARAM::default(),
                    LPARAM::default(),
                );
            }
        }
        if let Some(thread) = self.hook_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.writer_thread.take() {
            let _ = thread.join();
        }
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
    if code >= 0 && matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
        let key = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if key.flags.0 & LLKHF_INJECTED.0 == 0 && is_text_entry_key(key.vkCode) {
            if let Some(sender) = KEYBOARD_EVENTS.get() {
                let _ = sender.try_send(KeyboardMessage::Key(unix_millis()));
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

fn is_text_entry_key(virtual_key: u32) -> bool {
    matches!(
        virtual_key,
        0x20 | 0x30..=0x5a | 0x60..=0x6f | 0xba..=0xc0 | 0xdb..=0xde
    )
}

fn writer_loop(
    receiver: Receiver<KeyboardMessage>,
    path: &Path,
    health: &KeyboardHealth,
    recording: &AtomicBool,
) {
    let mut pending = BTreeMap::<u64, u64>::new();
    loop {
        match receiver.recv_timeout(FLUSH_INTERVAL) {
            Ok(KeyboardMessage::Key(timestamp)) => {
                if recording.load(Ordering::Acquire) {
                    *pending
                        .entry(timestamp / MINUTE_MILLIS * MINUTE_MILLIS)
                        .or_default() += 1;
                }
            }
            Ok(KeyboardMessage::Shutdown) => {
                flush_pending(path, &mut pending, health);
                break;
            }
            Err(RecvTimeoutError::Timeout) => flush_pending(path, &mut pending, health),
            Err(RecvTimeoutError::Disconnected) => {
                flush_pending(path, &mut pending, health);
                break;
            }
        }
    }
}

fn flush_pending(path: &Path, pending: &mut BTreeMap<u64, u64>, health: &KeyboardHealth) {
    if pending.is_empty() {
        return;
    }
    let result = (|| -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| error.to_string())?;
        let mut writer = BufWriter::new(file);
        for (start, key_strokes) in pending.iter() {
            serde_json::to_writer(
                &mut writer,
                &KeyboardRecord {
                    version: 1,
                    start: *start,
                    key_strokes: *key_strokes,
                },
            )
            .map_err(|error| error.to_string())?;
            writer.write_all(b"\n").map_err(|error| error.to_string())?;
        }
        writer.flush().map_err(|error| error.to_string())
    })();
    match result {
        Ok(()) => {
            pending.clear();
            health.mark_write(unix_millis());
        }
        Err(error) => health.set_error(format!("键盘计数写入失败：{error}")),
    }
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
        for line in reader.lines().map_while(Result::ok) {
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
        source: "iTime Windows 键盘字符键计数",
        updated_at: modified_millis(path).max(health.last_write_at.load(Ordering::Acquire)),
        skipped_records,
        buckets,
        capabilities: KeyboardCapabilities {
            content_captured: false,
            key_identity_captured: false,
            direct_key_count: true,
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

    #[test]
    fn counts_character_keys_without_modifiers_or_navigation() {
        for key in [0x20, 0x30, 0x41, 0x5a, 0x60, 0x6e, 0xba, 0xde] {
            assert!(is_text_entry_key(key), "expected {key:#x} to count");
        }
        for key in [0x08, 0x0d, 0x10, 0x11, 0x25, 0x70] {
            assert!(!is_text_entry_key(key), "expected {key:#x} not to count");
        }
    }

    #[test]
    fn aggregates_duplicate_minute_records_without_key_identity() {
        let path = std::env::temp_dir().join(format!(
            "itime-keyboard-{}-{}.jsonl",
            std::process::id(),
            unix_millis()
        ));
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
