use chrono::{Local, TimeZone};
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
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
        System::Threading::GetCurrentThreadId,
        UI::WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
            UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL,
            WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

const MINUTE_MILLIS: u64 = 60_000;
const FLUSH_INTERVAL: Duration = Duration::from_secs(3);
const EVENT_QUEUE_CAPACITY: usize = 4_096;
const DETAIL_VERSION: u8 = 1;

#[derive(Clone, Debug)]
struct KeyObservation {
    timestamp: u64,
    key: String,
    shortcut: Option<String>,
    text_entry: bool,
}

#[derive(Debug)]
enum KeyboardMessage {
    Key(KeyObservation),
    Shutdown,
}

static KEYBOARD_EVENTS: OnceLock<SyncSender<KeyboardMessage>> = OnceLock::new();
static MODIFIER_KEYS: AtomicU64 = AtomicU64::new(0);

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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardDetailCounts {
    keys: BTreeMap<String, u64>,
    shortcuts: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardDetailFile {
    version: u8,
    started_at: u64,
    days: BTreeMap<String, KeyboardDetailCounts>,
}

impl Default for KeyboardDetailFile {
    fn default() -> Self {
        Self {
            version: DETAIL_VERSION,
            started_at: 0,
            days: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyboardKeyCount {
    key: String,
    count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyboardShortcutCount {
    shortcut: String,
    count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyboardDetailDay {
    date: String,
    keys: Vec<KeyboardKeyCount>,
    shortcuts: Vec<KeyboardShortcutCount>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardCapabilities {
    content_captured: bool,
    sequence_captured: bool,
    key_identity_captured: bool,
    shortcut_counts_captured: bool,
    direct_key_count: bool,
    granularity: &'static str,
    detail_granularity: &'static str,
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
    detail_started_at: Option<u64>,
    detail_days: Vec<KeyboardDetailDay>,
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
    detail_path: PathBuf,
    health: Arc<KeyboardHealth>,
}

impl KeyboardService {
    pub(crate) fn new() -> Self {
        Self {
            path: keyboard_path(),
            detail_path: keyboard_detail_path(),
            health: Arc::new(KeyboardHealth::default()),
        }
    }

    fn snapshot(&self, start: u64, end: u64) -> Result<KeyboardSnapshot, String> {
        read_snapshot(&self.path, &self.detail_path, start, end, &self.health)
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
        MODIFIER_KEYS.store(0, Ordering::Release);
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let _ = KEYBOARD_EVENTS.set(sender.clone());

        let writer_path = service.path.clone();
        let detail_path = service.detail_path.clone();
        let writer_health = service.health.clone();
        let writer_thread = thread::spawn(move || {
            writer_loop(
                receiver,
                &writer_path,
                &detail_path,
                &writer_health,
                &recording,
            )
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
    if code >= 0 {
        let key = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if key.flags.0 & LLKHF_INJECTED.0 == 0 {
            let message = wparam.0 as u32;
            if matches!(message, WM_KEYUP | WM_SYSKEYUP) {
                update_modifier(key.vkCode, false);
            } else if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                let is_modifier = update_modifier(key.vkCode, true);
                if let Some(label) = key_label(key.vkCode) {
                    let observation = KeyObservation {
                        timestamp: unix_millis(),
                        shortcut: (!is_modifier).then(|| shortcut_label(&label)).flatten(),
                        key: label,
                        text_entry: is_text_entry_key(key.vkCode),
                    };
                    if let Some(sender) = KEYBOARD_EVENTS.get() {
                        let _ = sender.try_send(KeyboardMessage::Key(observation));
                    }
                }
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

fn modifier_bit(virtual_key: u32) -> Option<u64> {
    match virtual_key {
        0x10 => Some(1 << 0),
        0xa0 => Some(1 << 1),
        0xa1 => Some(1 << 2),
        0x11 => Some(1 << 3),
        0xa2 => Some(1 << 4),
        0xa3 => Some(1 << 5),
        0x12 => Some(1 << 6),
        0xa4 => Some(1 << 7),
        0xa5 => Some(1 << 8),
        0x5b => Some(1 << 9),
        0x5c => Some(1 << 10),
        _ => None,
    }
}

fn update_modifier(virtual_key: u32, pressed: bool) -> bool {
    let Some(bit) = modifier_bit(virtual_key) else {
        return false;
    };
    if pressed {
        MODIFIER_KEYS.fetch_or(bit, Ordering::AcqRel);
    } else {
        MODIFIER_KEYS.fetch_and(!bit, Ordering::AcqRel);
    }
    true
}

fn modifier_active(mask: u64, keys: &[u32]) -> bool {
    keys.iter()
        .filter_map(|key| modifier_bit(*key))
        .any(|bit| mask & bit != 0)
}

fn shortcut_label(key: &str) -> Option<String> {
    let mask = MODIFIER_KEYS.load(Ordering::Acquire);
    let ctrl = modifier_active(mask, &[0x11, 0xa2, 0xa3]);
    let alt = modifier_active(mask, &[0x12, 0xa4, 0xa5]);
    let shift = modifier_active(mask, &[0x10, 0xa0, 0xa1]);
    let windows = modifier_active(mask, &[0x5b, 0x5c]);
    if !ctrl && !alt && !windows {
        return None;
    }
    let mut parts = Vec::with_capacity(5);
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    if windows {
        parts.push("Win");
    }
    parts.push(key);
    Some(parts.join(" + "))
}

fn key_label(virtual_key: u32) -> Option<String> {
    let label = match virtual_key {
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0d => "Enter".into(),
        0x10 | 0xa0 | 0xa1 => "Shift".into(),
        0x11 | 0xa2 | 0xa3 => "Ctrl".into(),
        0x12 | 0xa4 | 0xa5 => "Alt".into(),
        0x14 => "Caps".into(),
        0x20 => "Space".into(),
        0x5b | 0x5c => "Win".into(),
        0x30..=0x39 | 0x41..=0x5a => char::from_u32(virtual_key)?.to_string(),
        0x60..=0x69 => char::from_u32(0x30 + virtual_key - 0x60)?.to_string(),
        0x6a => "*".into(),
        0x6b => "+".into(),
        0x6d => "-".into(),
        0x6e => ".".into(),
        0x6f => "/".into(),
        0xba => ";".into(),
        0xbb => "=".into(),
        0xbc => ",".into(),
        0xbd => "-".into(),
        0xbe => ".".into(),
        0xbf => "/".into(),
        0xc0 => "`".into(),
        0xdb => "[".into(),
        0xdc => "\\".into(),
        0xdd => "]".into(),
        0xde => "'".into(),
        _ => return None,
    };
    Some(label)
}

fn is_text_entry_key(virtual_key: u32) -> bool {
    matches!(
        virtual_key,
        0x20 | 0x30..=0x5a | 0x60..=0x6f | 0xba..=0xc0 | 0xdb..=0xde
    )
}

#[derive(Default)]
struct DetailAccumulator {
    started_at: Option<u64>,
    days: BTreeMap<String, KeyboardDetailCounts>,
}

impl DetailAccumulator {
    fn record(&mut self, observation: &KeyObservation) {
        self.started_at = Some(self.started_at.map_or(observation.timestamp, |value| {
            value.min(observation.timestamp)
        }));
        let day = self
            .days
            .entry(local_date_key(observation.timestamp))
            .or_default();
        *day.keys.entry(observation.key.clone()).or_default() += 1;
        if let Some(shortcut) = &observation.shortcut {
            *day.shortcuts.entry(shortcut.clone()).or_default() += 1;
        }
    }

    fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    fn clear(&mut self) {
        self.started_at = None;
        self.days.clear();
    }
}

fn writer_loop(
    receiver: Receiver<KeyboardMessage>,
    path: &Path,
    detail_path: &Path,
    health: &KeyboardHealth,
    recording: &AtomicBool,
) {
    let mut pending = BTreeMap::<u64, u64>::new();
    let mut detail_pending = DetailAccumulator::default();
    loop {
        match receiver.recv_timeout(FLUSH_INTERVAL) {
            Ok(KeyboardMessage::Key(observation)) => {
                if recording.load(Ordering::Acquire) {
                    if observation.text_entry {
                        *pending
                            .entry(observation.timestamp / MINUTE_MILLIS * MINUTE_MILLIS)
                            .or_default() += 1;
                    }
                    detail_pending.record(&observation);
                }
            }
            Ok(KeyboardMessage::Shutdown) => {
                flush_pending(path, detail_path, &mut pending, &mut detail_pending, health);
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                flush_pending(path, detail_path, &mut pending, &mut detail_pending, health)
            }
            Err(RecvTimeoutError::Disconnected) => {
                flush_pending(path, detail_path, &mut pending, &mut detail_pending, health);
                break;
            }
        }
    }
}

fn flush_pending(
    path: &Path,
    detail_path: &Path,
    pending: &mut BTreeMap<u64, u64>,
    detail_pending: &mut DetailAccumulator,
    health: &KeyboardHealth,
) {
    let mut errors = Vec::new();
    let mut wrote = false;
    if !detail_pending.is_empty() {
        match write_detail_file(detail_path, detail_pending) {
            Ok(()) => {
                detail_pending.clear();
                wrote = true;
            }
            Err(error) => errors.push(error),
        }
    }
    if !pending.is_empty() {
        match append_minute_counts(path, pending) {
            Ok(()) => {
                pending.clear();
                wrote = true;
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        if wrote {
            health.mark_write(unix_millis());
        }
    } else {
        health.set_error(format!("键盘统计写入失败：{}", errors.join("；")));
    }
}

fn append_minute_counts(path: &Path, pending: &BTreeMap<u64, u64>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    for (start, key_strokes) in pending {
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
}

fn write_detail_file(path: &Path, pending: &DetailAccumulator) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut detail = if path.is_file() {
        serde_json::from_reader::<_, KeyboardDetailFile>(
            File::open(path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("逐键聚合文件无法读取：{error}"))?
    } else {
        KeyboardDetailFile::default()
    };
    if detail.version != DETAIL_VERSION {
        return Err("逐键聚合文件版本不受支持".into());
    }
    if let Some(started_at) = pending.started_at {
        detail.started_at = if detail.started_at == 0 {
            started_at
        } else {
            detail.started_at.min(started_at)
        };
    }
    for (date, counts) in &pending.days {
        let target = detail.days.entry(date.clone()).or_default();
        for (key, count) in &counts.keys {
            *target.keys.entry(key.clone()).or_default() += count;
        }
        for (shortcut, count) in &counts.shortcuts {
            *target.shortcuts.entry(shortcut.clone()).or_default() += count;
        }
    }
    let temporary = path.with_extension("json.tmp");
    let file = File::create(&temporary).map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &detail).map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|error| error.to_string())?;
    replace_file(&temporary, path)
}

fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source_wide.as_ptr()),
            PCWSTR(target_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|error| error.to_string())
}

fn read_snapshot(
    path: &Path,
    detail_path: &Path,
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

    let start_date = local_date_key(start);
    let end_date = local_date_key(end.saturating_sub(1));
    let mut detail_started_at = None;
    let mut detail_days = Vec::new();
    if detail_path.is_file() {
        match serde_json::from_reader::<_, KeyboardDetailFile>(
            File::open(detail_path).map_err(|error| error.to_string())?,
        ) {
            Ok(detail) if detail.version == DETAIL_VERSION => {
                detail_started_at = (detail.started_at > 0).then_some(detail.started_at);
                detail_days = detail
                    .days
                    .into_iter()
                    .filter(|(date, _)| date >= &start_date && date <= &end_date)
                    .map(|(date, counts)| KeyboardDetailDay {
                        date,
                        keys: sorted_key_counts(counts.keys),
                        shortcuts: sorted_shortcut_counts(counts.shortcuts),
                    })
                    .collect();
            }
            Ok(_) | Err(_) => skipped_records += 1,
        }
    }

    Ok(KeyboardSnapshot {
        source: "iTime Windows 键盘本地聚合统计",
        updated_at: modified_millis(path)
            .max(modified_millis(detail_path))
            .max(health.last_write_at.load(Ordering::Acquire)),
        skipped_records,
        buckets,
        detail_started_at,
        detail_days,
        capabilities: KeyboardCapabilities {
            content_captured: false,
            sequence_captured: false,
            key_identity_captured: true,
            shortcut_counts_captured: true,
            direct_key_count: true,
            granularity: "minute",
            detail_granularity: "day",
            timezone_semantics: "local-time",
            historical_backfill: false,
        },
        health: health.snapshot(),
    })
}

fn sorted_key_counts(values: BTreeMap<String, u64>) -> Vec<KeyboardKeyCount> {
    let mut values: Vec<_> = values
        .into_iter()
        .map(|(key, count)| KeyboardKeyCount { key, count })
        .collect();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    values
}

fn sorted_shortcut_counts(values: BTreeMap<String, u64>) -> Vec<KeyboardShortcutCount> {
    let mut values: Vec<_> = values
        .into_iter()
        .map(|(shortcut, count)| KeyboardShortcutCount { shortcut, count })
        .collect();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.shortcut.cmp(&right.shortcut))
    });
    values
}

fn local_date_key(timestamp: u64) -> String {
    let timestamp = i64::try_from(timestamp).unwrap_or(i64::MAX);
    Local
        .timestamp_millis_opt(timestamp)
        .single()
        .unwrap_or_else(Local::now)
        .format("%Y-%m-%d")
        .to_string()
}

fn keyboard_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("iTime")
        .join("Data")
}

fn keyboard_path() -> PathBuf {
    keyboard_data_dir().join("keyboard-v1.jsonl")
}

fn keyboard_detail_path() -> PathBuf {
    keyboard_data_dir().join("keyboard-detail-v1.json")
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

    fn temp_paths(label: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "itime-keyboard-{label}-{}-{}",
            std::process::id(),
            unix_millis()
        ));
        let _ = fs::create_dir_all(&root);
        (
            root.join("keyboard-v1.jsonl"),
            root.join("keyboard-detail-v1.json"),
        )
    }

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
    fn normalizes_keys_and_shortcuts_without_raw_codes() {
        MODIFIER_KEYS.store(0, Ordering::Release);
        assert_eq!(key_label(0x41).as_deref(), Some("A"));
        assert_eq!(key_label(0x08).as_deref(), Some("Backspace"));
        assert!(update_modifier(0xa2, true));
        assert_eq!(shortcut_label("C").as_deref(), Some("Ctrl + C"));
        assert!(update_modifier(0xa0, true));
        assert_eq!(shortcut_label("S").as_deref(), Some("Ctrl + Shift + S"));
        update_modifier(0xa2, false);
        update_modifier(0xa0, false);
    }

    #[test]
    fn aggregates_legacy_minutes_and_daily_details_without_sequence() {
        let (path, detail_path) = temp_paths("aggregate");
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
        let mut detail = DetailAccumulator::default();
        detail.record(&KeyObservation {
            timestamp: 60_000,
            key: "A".into(),
            shortcut: None,
            text_entry: true,
        });
        detail.record(&KeyObservation {
            timestamp: 61_000,
            key: "C".into(),
            shortcut: Some("Ctrl + C".into()),
            text_entry: true,
        });
        write_detail_file(&detail_path, &detail).unwrap();

        let health = KeyboardHealth::default();
        let snapshot = read_snapshot(&path, &detail_path, 0, 180_000, &health).unwrap();
        assert_eq!(snapshot.buckets.len(), 2);
        assert_eq!(snapshot.buckets[0].key_strokes, 7);
        assert_eq!(snapshot.detail_days.len(), 1);
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("Ctrl + C"));
        assert!(json.contains("\"contentCaptured\":false"));
        assert!(json.contains("\"sequenceCaptured\":false"));
        for forbidden in ["vkCode", "rawKey", "inputText", "eventSequence"] {
            assert!(!json.contains(forbidden));
        }
        let root = path.parent().unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn paused_recording_does_not_accumulate_details() {
        let (path, detail_path) = temp_paths("paused");
        let recording = AtomicBool::new(false);
        let health = KeyboardHealth::default();
        let (sender, receiver) = mpsc::sync_channel(2);
        sender
            .send(KeyboardMessage::Key(KeyObservation {
                timestamp: unix_millis(),
                key: "A".into(),
                shortcut: None,
                text_entry: true,
            }))
            .unwrap();
        sender.send(KeyboardMessage::Shutdown).unwrap();
        writer_loop(receiver, &path, &detail_path, &health, &recording);
        assert!(!path.exists());
        assert!(!detail_path.exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn persists_cross_day_counts_with_atomic_replacement() {
        let (_path, detail_path) = temp_paths("cross-day");
        let first_timestamp = 60_000;
        let second_timestamp = first_timestamp + 86_400_000;
        let mut first = DetailAccumulator::default();
        first.record(&KeyObservation {
            timestamp: second_timestamp,
            key: "B".into(),
            shortcut: None,
            text_entry: true,
        });
        write_detail_file(&detail_path, &first).unwrap();
        let mut second = DetailAccumulator::default();
        second.record(&KeyObservation {
            timestamp: first_timestamp,
            key: "A".into(),
            shortcut: Some("Ctrl + A".into()),
            text_entry: true,
        });
        write_detail_file(&detail_path, &second).unwrap();

        let persisted: KeyboardDetailFile =
            serde_json::from_reader(File::open(&detail_path).unwrap()).unwrap();
        assert_eq!(persisted.started_at, first_timestamp);
        assert_eq!(persisted.days.len(), 2);
        assert!(!detail_path.with_extension("json.tmp").exists());
        let _ = fs::remove_dir_all(detail_path.parent().unwrap());
    }
}
