use chrono::DateTime;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;

const DAY_MILLIS: u64 = 24 * 60 * 60 * 1_000;
const ACTIVE_GRACE_MILLIS: u64 = 5 * 60 * 1_000;
const MAX_PROVIDER_FILES: usize = 2_048;

#[derive(Clone, Copy)]
enum ProviderKind {
    Codex,
    Claude,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderInterval {
    version: u8,
    start: u64,
    end: u64,
    provider: &'static str,
    tool_id: &'static str,
    tool_name: &'static str,
    agent_id: String,
    task_id: String,
    status: &'static str,
    basis: &'static str,
    confidence: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderCapabilities {
    content_captured: bool,
    codex_task_events: bool,
    claude_turn_events: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderActivitySnapshot {
    source: &'static str,
    updated_at: u128,
    scanned_files: usize,
    skipped_files: usize,
    intervals: Vec<ProviderInterval>,
    capabilities: ProviderCapabilities,
}

#[derive(Clone)]
struct CachedFile {
    length: u64,
    modified_at: u64,
    intervals: Vec<ProviderInterval>,
}

#[derive(Clone)]
pub(crate) struct ProviderActivityService {
    cache: Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
}

impl ProviderActivityService {
    pub(crate) fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn snapshot(&self, start: u64, end: u64) -> Result<ProviderActivitySnapshot, String> {
        if end <= start {
            return Err("Provider 活动查询区间无效".into());
        }

        let home = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .ok_or_else(|| "Windows 用户目录不可用".to_string())?;
        let codex_root = home.join(r".codex\sessions");
        let claude_root = home.join(r".claude\projects");
        let cutoff = start.saturating_sub(2 * DAY_MILLIS);
        let now = unix_millis();
        let mut candidates = Vec::new();
        collect_candidates(&codex_root, ProviderKind::Codex, cutoff, 6, &mut candidates);
        collect_candidates(
            &claude_root,
            ProviderKind::Claude,
            cutoff,
            6,
            &mut candidates,
        );
        candidates.sort_by(|left, right| right.2.cmp(&left.2));
        candidates.truncate(MAX_PROVIDER_FILES);

        let mut intervals = Vec::new();
        let mut skipped_files = 0;
        for (path, kind, _, _) in &candidates {
            match self.load_file(path, *kind, now) {
                Ok(items) => intervals.extend(items),
                Err(_) => skipped_files += 1,
            }
        }

        let mut seen = HashSet::new();
        intervals.retain(|interval| {
            interval.start < end
                && interval.end > start
                && seen.insert((
                    interval.provider,
                    interval.task_id.clone(),
                    interval.start,
                    interval.end,
                ))
        });
        intervals.sort_by_key(|interval| (interval.start, interval.end));

        Ok(ProviderActivitySnapshot {
            source: "Codex 与 Claude Code 本机会话时间事件",
            updated_at: u128::from(now),
            scanned_files: candidates.len(),
            skipped_files,
            intervals,
            capabilities: ProviderCapabilities {
                content_captured: false,
                codex_task_events: codex_root.is_dir(),
                claude_turn_events: claude_root.is_dir(),
            },
        })
    }

    fn load_file(
        &self,
        path: &Path,
        kind: ProviderKind,
        now: u64,
    ) -> Result<Vec<ProviderInterval>, String> {
        let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(system_millis)
            .unwrap_or(0);
        if let Some(cached) = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(path)
            .filter(|cached| cached.length == metadata.len() && cached.modified_at == modified_at)
            .cloned()
        {
            return Ok(cached.intervals);
        }

        let intervals = match kind {
            ProviderKind::Codex => parse_codex_file(path, modified_at, now),
            ProviderKind::Claude => parse_claude_file(path, modified_at, now),
        }
        .map_err(|error| error.to_string())?;
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                path.to_path_buf(),
                CachedFile {
                    length: metadata.len(),
                    modified_at,
                    intervals: intervals.clone(),
                },
            );
        Ok(intervals)
    }
}

impl Default for ProviderActivityService {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub(crate) fn get_provider_activity_snapshot(
    providers: State<'_, ProviderActivityService>,
    start: u64,
    end: u64,
) -> Result<ProviderActivitySnapshot, String> {
    providers.snapshot(start, end)
}

fn collect_candidates(
    root: &Path,
    kind: ProviderKind,
    modified_after: u64,
    depth: u8,
    output: &mut Vec<(PathBuf, ProviderKind, u64, u64)>,
) {
    if depth == 0 || !root.is_dir() || output.len() >= MAX_PROVIDER_FILES * 2 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_candidates(&path, kind, modified_after, depth - 1, output);
            continue;
        }
        let accepted = match kind {
            ProviderKind::Codex => path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl")),
            ProviderKind::Claude => path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl")),
        };
        if !accepted {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(system_millis)
            .unwrap_or(0);
        if modified >= modified_after {
            output.push((path, kind, modified, metadata.len()));
        }
    }
}

fn parse_codex_file(
    path: &Path,
    modified_at: u64,
    now: u64,
) -> std::io::Result<Vec<ProviderInterval>> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = Vec::new();
    let mut open_start = None;
    let mut intervals = Vec::new();

    while reader.read_until(b'\n', &mut line)? > 0 {
        let has_lifecycle_event = contains_bytes(&line, b"task_started")
            || contains_bytes(&line, b"task_complete")
            || contains_bytes(&line, b"turn_aborted");
        if has_lifecycle_event {
            let value = std::str::from_utf8(&line).ok().and_then(parse_value);
            if let Some((kind, timestamp)) = value
                .as_ref()
                .and_then(|value| event_kind(value).zip(event_timestamp(value)))
            {
                match kind {
                    "task_started" => open_start = Some(timestamp),
                    "task_complete" | "turn_aborted" => {
                        if let Some(start) = open_start.take().filter(|start| timestamp > *start) {
                            intervals.push(codex_interval(
                                path,
                                start,
                                timestamp,
                                "completed",
                                if kind == "turn_aborted" {
                                    "Codex 本机会话 task_started/turn_aborted 时间事件"
                                } else {
                                    "Codex 本机会话 task_started/task_complete 时间事件"
                                },
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        line.clear();
    }

    if let Some(start) = open_start {
        let active = now.saturating_sub(modified_at) <= ACTIVE_GRACE_MILLIS;
        if active && now > start {
            intervals.push(codex_interval(
                path,
                start,
                now,
                "running",
                "Codex 本机会话 task_started 进行中时间事件",
            ));
        }
    }
    Ok(intervals)
}

fn codex_interval(
    path: &Path,
    start: u64,
    end: u64,
    status: &'static str,
    basis: &'static str,
) -> ProviderInterval {
    let id = stable_id("codex", path, start);
    ProviderInterval {
        version: 1,
        start,
        end,
        provider: "codex",
        tool_id: "codex",
        tool_name: "Codex",
        agent_id: id.clone(),
        task_id: id,
        status,
        basis,
        confidence: 0.99,
    }
}

fn parse_claude_file(
    path: &Path,
    modified_at: u64,
    now: u64,
) -> std::io::Result<Vec<ProviderInterval>> {
    let reader = BufReader::new(File::open(path)?);
    let mut intervals = Vec::new();
    let mut latest_human_start = None;
    let mut latest_completed_end = 0;
    let mut latest_end_turn = None;
    let mut last_timestamp = None;
    for line in reader.lines().map_while(Result::ok) {
        if !line.contains("turn_duration")
            && !line.contains("end_turn")
            && !line.contains("\"type\":\"user\"")
            && !line.contains("\"type\": \"user\"")
        {
            continue;
        }
        let Some(value) = parse_value(&line) else {
            continue;
        };
        let timestamp = event_timestamp(&value);
        if let Some(timestamp) = timestamp {
            last_timestamp =
                Some(last_timestamp.map_or(timestamp, |last: u64| last.max(timestamp)));
        }
        if is_claude_human_turn(&value) {
            latest_human_start = timestamp;
            continue;
        }
        if value.get("type").and_then(Value::as_str) == Some("assistant")
            && value
                .pointer("/message/stop_reason")
                .and_then(Value::as_str)
                == Some("end_turn")
        {
            latest_end_turn = timestamp;
            continue;
        }
        if value.get("type").and_then(Value::as_str) != Some("system")
            || value.get("subtype").and_then(Value::as_str) != Some("turn_duration")
        {
            continue;
        }
        let Some(end) = timestamp else {
            continue;
        };
        let Some(duration) = value.get("durationMs").and_then(Value::as_u64) else {
            continue;
        };
        if duration == 0 || duration > 7 * DAY_MILLIS {
            continue;
        }
        let start = end.saturating_sub(duration);
        if end <= start {
            continue;
        }
        latest_completed_end = latest_completed_end.max(end);
        let id = stable_id("claude", path, end);
        intervals.push(ProviderInterval {
            version: 1,
            start,
            end,
            provider: "claude",
            tool_id: "claude",
            tool_name: "Claude Code",
            agent_id: id.clone(),
            task_id: id,
            status: "completed",
            basis: "Claude Code 本机会话 turn_duration 时间事件",
            confidence: 0.99,
        });
    }

    if let Some(start) = latest_human_start.filter(|start| *start > latest_completed_end) {
        let completed = latest_end_turn.filter(|end| *end > start);
        let active = completed.is_none() && now.saturating_sub(modified_at) <= ACTIVE_GRACE_MILLIS;
        let end = completed
            .or_else(|| active.then_some(now))
            .or(last_timestamp)
            .unwrap_or(start);
        if end > start {
            let id = stable_id("claude", path, start);
            intervals.push(ProviderInterval {
                version: 1,
                start,
                end,
                provider: "claude",
                tool_id: "claude",
                tool_name: "Claude Code",
                agent_id: id.clone(),
                task_id: id,
                status: if active { "running" } else { "completed" },
                basis: "Claude Code 本机会话 user/end_turn 时间事件",
                confidence: 0.9,
            });
        }
    }
    Ok(intervals)
}

fn is_claude_human_turn(value: &Value) -> bool {
    if value.get("type").and_then(Value::as_str) != Some("user")
        || value.pointer("/message/role").and_then(Value::as_str) != Some("user")
    {
        return false;
    }
    match value.pointer("/message/content") {
        Some(Value::String(_)) => true,
        Some(Value::Array(items)) => items
            .iter()
            .any(|item| item.get("type").and_then(Value::as_str) == Some("text")),
        _ => false,
    }
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn parse_value(line: &str) -> Option<Value> {
    serde_json::from_str(line.trim()).ok()
}

fn event_kind(value: &Value) -> Option<&str> {
    (value.get("type").and_then(Value::as_str) == Some("event_msg"))
        .then(|| value.pointer("/payload/type").and_then(Value::as_str))
        .flatten()
}

fn event_timestamp(value: &Value) -> Option<u64> {
    let timestamp = value.get("timestamp")?.as_str()?;
    let parsed = DateTime::parse_from_rfc3339(timestamp)
        .ok()?
        .timestamp_millis();
    u64::try_from(parsed).ok()
}

fn stable_id(provider: &str, path: &Path, timestamp: u64) -> String {
    let mut digest = Sha256::new();
    digest.update(provider.as_bytes());
    digest.update(path.to_string_lossy().as_bytes());
    digest.update(timestamp.to_le_bytes());
    hex::encode(&digest.finalize()[..8])
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
            "itime-provider-{name}-{}-{}.jsonl",
            std::process::id(),
            unix_millis()
        ))
    }

    #[test]
    fn reads_codex_task_boundaries_without_content_fields() {
        let path = fixture_path("codex");
        fs::write(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T01:00:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:03:00.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":"ignored"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:05:00.000Z","type":"event_msg","payload":{"type":"task_complete"}}"#,
                "\n"
            ),
        )
        .unwrap();
        let intervals = parse_codex_file(&path, 0, 0).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].end - intervals[0].start, 5 * 60_000);
        let json = serde_json::to_string(&intervals[0]).unwrap();
        assert!(!json.contains("content"));
    }

    #[test]
    fn keeps_codex_tasks_separate_and_excludes_idle_gaps() {
        let path = fixture_path("codex-multiple");
        fs::write(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T01:00:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:05:00.000Z","type":"event_msg","payload":{"type":"task_complete"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T02:00:00.000Z","type":"response_item","payload":{"type":"message","content":"ignored"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T02:10:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T02:12:00.000Z","type":"event_msg","payload":{"type":"turn_aborted"}}"#,
                "\n"
            ),
        )
        .unwrap();
        let intervals = parse_codex_file(&path, 0, 0).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(intervals.len(), 2);
        assert_eq!(
            intervals
                .iter()
                .map(|item| item.end - item.start)
                .sum::<u64>(),
            7 * 60_000
        );
        assert_ne!(intervals[0].task_id, intervals[1].task_id);
        assert!(intervals[1].basis.contains("turn_aborted"));
    }

    #[test]
    fn reads_claude_turn_duration_as_provider_work() {
        let path = fixture_path("claude");
        fs::write(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T02:10:00.000Z","type":"system","subtype":"turn_duration","durationMs":120000}"#,
                "\n"
            ),
        )
        .unwrap();
        let intervals = parse_claude_file(&path, 0, 0).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].end - intervals[0].start, 120_000);
        assert_eq!(intervals[0].tool_id, "claude");
    }
}
