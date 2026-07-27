use crate::settings::{self, ProviderConsent};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, Metadata},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;

const DAY_MILLIS: u64 = 24 * 60 * 60 * 1_000;
const ACTIVE_GRACE_MILLIS: u64 = 5 * 60 * 1_000;
const MAX_PROVIDER_FILES: usize = 2_048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderDiagnostics {
    candidate_files: usize,
    selected_files: usize,
    parsed_files: usize,
    cache_hits: usize,
    bad_lines: usize,
    bad_events: usize,
    read_failures: usize,
    permission_failures: usize,
}

impl ProviderDiagnostics {
    fn add_parse(&mut self, diagnostics: &ParseDiagnostics) {
        self.bad_lines += diagnostics.bad_lines;
        self.bad_events += diagnostics.bad_events;
    }

    fn has_degradation(&self) -> bool {
        self.bad_lines > 0
            || self.bad_events > 0
            || self.read_failures > 0
            || self.permission_failures > 0
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderActivitySnapshot {
    source: &'static str,
    status: &'static str,
    updated_at: u128,
    scanned_files: usize,
    skipped_files: usize,
    intervals: Vec<ProviderInterval>,
    consent: ProviderConsent,
    diagnostics: ProviderDiagnostics,
    capabilities: ProviderCapabilities,
}

#[derive(Clone, Debug, Default)]
struct ParseDiagnostics {
    bad_lines: usize,
    bad_events: usize,
}

#[derive(Clone)]
struct OpenInterval {
    start: u64,
    provider: ProviderKind,
}

#[derive(Clone, Default)]
struct ParsedFileFacts {
    completed: Vec<ProviderInterval>,
    open: Option<OpenInterval>,
}

#[derive(Clone)]
struct ParsedFile {
    facts: ParsedFileFacts,
    diagnostics: ParseDiagnostics,
}

#[derive(Clone)]
struct CachedFile {
    length: u64,
    modified_at: u64,
    parsed: ParsedFile,
}

#[derive(Clone, Debug)]
struct Candidate {
    path: PathBuf,
    kind: ProviderKind,
    modified_at: u64,
    length: u64,
}

#[derive(Clone)]
pub(crate) struct ProviderActivityService {
    cache: Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
    consent: Arc<Mutex<ProviderConsent>>,
}

impl ProviderActivityService {
    pub(crate) fn new(consent: ProviderConsent) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            consent: Arc::new(Mutex::new(consent)),
        }
    }

    fn consent(&self) -> ProviderConsent {
        self.consent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn set_consent(&self, consent: ProviderConsent) -> Result<ProviderConsent, String> {
        consent.validate()?;
        settings::save_provider_consent(consent.clone())?;
        *self
            .consent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = consent.clone();
        if !consent.codex_enabled && !consent.claude_enabled {
            self.cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clear();
        }
        Ok(consent)
    }

    fn snapshot(&self, start: u64, end: u64) -> Result<ProviderActivitySnapshot, String> {
        let consent = self.consent();
        if end <= start {
            return Err("Provider 活动查询区间无效".into());
        }

        // This gate intentionally runs before USERPROFILE is resolved or any provider path is read.
        if !consent.codex_enabled && !consent.claude_enabled {
            return Ok(disabled_snapshot(consent));
        }

        let home = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .ok_or_else(|| "Windows 用户目录不可用".to_string())?;
        self.snapshot_with_home(start, end, unix_millis(), &home, consent)
    }

    fn snapshot_with_home(
        &self,
        start: u64,
        end: u64,
        now: u64,
        home: &Path,
        consent: ProviderConsent,
    ) -> Result<ProviderActivitySnapshot, String> {
        if end <= start {
            return Err("Provider 活动查询区间无效".into());
        }
        if !consent.codex_enabled && !consent.claude_enabled {
            return Ok(disabled_snapshot(consent));
        }

        let codex_root = home.join(r".codex\sessions");
        let claude_root = home.join(r".claude\projects");
        let cutoff = start.saturating_sub(2 * DAY_MILLIS);
        let mut diagnostics = ProviderDiagnostics::default();
        let mut candidates = Vec::new();
        let codex_available = consent.codex_enabled
            && collect_candidates(
                &codex_root,
                ProviderKind::Codex,
                cutoff,
                6,
                &mut candidates,
                &mut diagnostics,
            );
        let claude_available = consent.claude_enabled
            && collect_candidates(
                &claude_root,
                ProviderKind::Claude,
                cutoff,
                6,
                &mut candidates,
                &mut diagnostics,
            );
        diagnostics.candidate_files = candidates.len();
        select_newest_candidates(&mut candidates);
        diagnostics.selected_files = candidates.len();

        let selected_paths = candidates
            .iter()
            .map(|candidate| candidate.path.clone())
            .collect::<HashSet<_>>();
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|path, _| selected_paths.contains(path));

        let mut intervals = Vec::new();
        let mut skipped_files = 0;
        for candidate in &candidates {
            match self.load_file(candidate) {
                Ok((parsed, cache_hit)) => {
                    if cache_hit {
                        diagnostics.cache_hits += 1;
                    } else {
                        diagnostics.parsed_files += 1;
                    }
                    diagnostics.add_parse(&parsed.diagnostics);
                    intervals.extend(materialize_facts(
                        &candidate.path,
                        &parsed.facts,
                        candidate.modified_at,
                        now,
                        &mut diagnostics,
                    ));
                }
                Err(error) => {
                    skipped_files += 1;
                    record_io_error(&mut diagnostics, &error);
                    self.cache
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .remove(&candidate.path);
                }
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

        let any_enabled_root = (consent.codex_enabled && codex_available)
            || (consent.claude_enabled && claude_available);
        let enabled_root_missing = (consent.codex_enabled && !codex_available)
            || (consent.claude_enabled && !claude_available);
        let status = if !any_enabled_root {
            "unavailable"
        } else if enabled_root_missing || diagnostics.has_degradation() {
            "partial"
        } else {
            "ready"
        };

        Ok(ProviderActivitySnapshot {
            source: "已授权的 Codex 与 Claude Code 本机会话时间事件",
            status,
            updated_at: u128::from(now),
            scanned_files: candidates.len(),
            skipped_files,
            intervals,
            consent,
            diagnostics,
            capabilities: ProviderCapabilities {
                content_captured: false,
                codex_task_events: codex_available,
                claude_turn_events: claude_available,
            },
        })
    }

    fn load_file(&self, candidate: &Candidate) -> io::Result<(ParsedFile, bool)> {
        let metadata = fs::metadata(&candidate.path)?;
        let modified_at = metadata_modified(&metadata);
        if let Some(cached) = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&candidate.path)
            .filter(|cached| cached.length == metadata.len() && cached.modified_at == modified_at)
            .cloned()
        {
            return Ok((cached.parsed, true));
        }

        let parsed = match candidate.kind {
            ProviderKind::Codex => parse_codex_file(&candidate.path),
            ProviderKind::Claude => parse_claude_file(&candidate.path),
        }?;
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                candidate.path.clone(),
                CachedFile {
                    length: metadata.len(),
                    modified_at,
                    parsed: parsed.clone(),
                },
            );
        Ok((parsed, false))
    }
}

fn disabled_snapshot(consent: ProviderConsent) -> ProviderActivitySnapshot {
    ProviderActivitySnapshot {
        source: "Provider 本机会话读取未授权",
        status: "disabled",
        updated_at: u128::from(unix_millis()),
        scanned_files: 0,
        skipped_files: 0,
        intervals: Vec::new(),
        consent,
        diagnostics: ProviderDiagnostics::default(),
        capabilities: ProviderCapabilities {
            content_captured: false,
            codex_task_events: false,
            claude_turn_events: false,
        },
    }
}

impl Default for ProviderActivityService {
    fn default() -> Self {
        Self::new(ProviderConsent::default())
    }
}

#[tauri::command]
pub(crate) fn get_provider_consent(
    providers: State<'_, ProviderActivityService>,
) -> ProviderConsent {
    providers.consent()
}

#[tauri::command]
pub(crate) fn set_provider_consent(
    providers: State<'_, ProviderActivityService>,
    consent: ProviderConsent,
) -> Result<ProviderConsent, String> {
    providers.set_consent(consent)
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
    output: &mut Vec<Candidate>,
    diagnostics: &mut ProviderDiagnostics,
) -> bool {
    if depth == 0 {
        return false;
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return false,
        Err(error) => {
            record_io_error(diagnostics, &error);
            return false;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                record_io_error(diagnostics, &error);
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                record_io_error(diagnostics, &error);
                continue;
            }
        };
        if file_type.is_dir() {
            collect_candidates(&path, kind, modified_after, depth - 1, output, diagnostics);
            continue;
        }
        if !provider_file_name(&path, kind) {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                record_io_error(diagnostics, &error);
                continue;
            }
        };
        let modified_at = metadata_modified(&metadata);
        if modified_at >= modified_after {
            output.push(Candidate {
                path,
                kind,
                modified_at,
                length: metadata.len(),
            });
        }
    }
    true
}

fn provider_file_name(path: &Path, kind: ProviderKind) -> bool {
    match kind {
        ProviderKind::Codex => path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl")),
        ProviderKind::Claude => path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl")),
    }
}

fn select_newest_candidates(candidates: &mut Vec<Candidate>) {
    candidates.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| right.length.cmp(&left.length))
            .then_with(|| left.path.cmp(&right.path))
    });
    candidates.truncate(MAX_PROVIDER_FILES);
}

fn parse_codex_file(path: &Path) -> io::Result<ParsedFile> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = Vec::new();
    let mut open_start = None;
    let mut facts = ParsedFileFacts::default();
    let mut diagnostics = ParseDiagnostics::default();

    while reader.read_until(b'\n', &mut line)? > 0 {
        let has_lifecycle_event = contains_bytes(&line, b"task_started")
            || contains_bytes(&line, b"task_complete")
            || contains_bytes(&line, b"turn_aborted");
        if has_lifecycle_event {
            match serde_json::from_slice::<WireEvent>(&line) {
                Ok(event) => {
                    let Some((kind, timestamp)) = codex_event(&event) else {
                        diagnostics.bad_events += 1;
                        line.clear();
                        continue;
                    };
                    match kind {
                        "task_started" => {
                            if open_start.replace(timestamp).is_some() {
                                diagnostics.bad_events += 1;
                            }
                        }
                        "task_complete" | "turn_aborted" => {
                            if let Some(start) =
                                open_start.take().filter(|start| timestamp > *start)
                            {
                                facts.completed.push(codex_interval(
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
                            } else {
                                diagnostics.bad_events += 1;
                            }
                        }
                        _ => {}
                    }
                }
                Err(_) => diagnostics.bad_lines += 1,
            }
        }
        line.clear();
    }
    facts.open = open_start.map(|start| OpenInterval {
        start,
        provider: ProviderKind::Codex,
    });
    Ok(ParsedFile { facts, diagnostics })
}

fn parse_claude_file(path: &Path) -> io::Result<ParsedFile> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = Vec::new();
    let mut facts = ParsedFileFacts::default();
    let mut diagnostics = ParseDiagnostics::default();
    let mut latest_human_start = None;
    let mut latest_completed_end = 0;
    let mut latest_end_turn = None;

    while reader.read_until(b'\n', &mut line)? > 0 {
        let relevant = contains_bytes(&line, b"turn_duration")
            || contains_bytes(&line, b"end_turn")
            || contains_bytes(&line, br#""type":"user""#)
            || contains_bytes(&line, br#""type": "user""#);
        if !relevant {
            line.clear();
            continue;
        }

        match serde_json::from_slice::<WireEvent>(&line) {
            Ok(event) => {
                let timestamp = parse_timestamp(event.timestamp.as_deref());
                if event.record_type.as_deref() == Some("user")
                    && event
                        .message
                        .as_ref()
                        .and_then(|message| message.role.as_deref())
                        == Some("user")
                {
                    if let Some(timestamp) = timestamp {
                        latest_human_start = Some(timestamp);
                    } else {
                        diagnostics.bad_events += 1;
                    }
                } else if event.record_type.as_deref() == Some("assistant")
                    && event
                        .message
                        .as_ref()
                        .and_then(|message| message.stop_reason.as_deref())
                        == Some("end_turn")
                {
                    if let Some(timestamp) = timestamp {
                        latest_end_turn = Some(timestamp);
                    } else {
                        diagnostics.bad_events += 1;
                    }
                } else if event.record_type.as_deref() == Some("system")
                    && event.subtype.as_deref() == Some("turn_duration")
                {
                    let Some(end) = timestamp else {
                        diagnostics.bad_events += 1;
                        line.clear();
                        continue;
                    };
                    let Some(duration) = event.duration_ms else {
                        diagnostics.bad_events += 1;
                        line.clear();
                        continue;
                    };
                    if duration == 0 || duration > 7 * DAY_MILLIS {
                        diagnostics.bad_events += 1;
                        line.clear();
                        continue;
                    }
                    let start = end.saturating_sub(duration);
                    if end <= start {
                        diagnostics.bad_events += 1;
                        line.clear();
                        continue;
                    }
                    latest_completed_end = latest_completed_end.max(end);
                    facts
                        .completed
                        .push(claude_interval(path, start, end, "completed", 0.99));
                } else {
                    diagnostics.bad_events += 1;
                }
            }
            Err(_) => diagnostics.bad_lines += 1,
        }
        line.clear();
    }

    if let Some(start) = latest_human_start.filter(|start| *start > latest_completed_end) {
        if let Some(end) = latest_end_turn.filter(|end| *end > start) {
            facts
                .completed
                .push(claude_interval(path, start, end, "completed", 0.9));
        } else {
            facts.open = Some(OpenInterval {
                start,
                provider: ProviderKind::Claude,
            });
        }
    }
    Ok(ParsedFile { facts, diagnostics })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireEvent {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    record_type: Option<String>,
    payload: Option<WirePayload>,
    subtype: Option<String>,
    duration_ms: Option<u64>,
    message: Option<WireMessageMetadata>,
}

#[derive(Deserialize)]
struct WirePayload {
    #[serde(rename = "type")]
    event_type: Option<String>,
}

#[derive(Deserialize)]
struct WireMessageMetadata {
    role: Option<String>,
    stop_reason: Option<String>,
}

fn codex_event(event: &WireEvent) -> Option<(&str, u64)> {
    if event.record_type.as_deref() != Some("event_msg") {
        return None;
    }
    let kind = event.payload.as_ref()?.event_type.as_deref()?;
    let timestamp = parse_timestamp(event.timestamp.as_deref())?;
    Some((kind, timestamp))
}

fn parse_timestamp(timestamp: Option<&str>) -> Option<u64> {
    let parsed = DateTime::parse_from_rfc3339(timestamp?)
        .ok()?
        .timestamp_millis();
    u64::try_from(parsed).ok()
}

fn materialize_facts(
    path: &Path,
    facts: &ParsedFileFacts,
    modified_at: u64,
    now: u64,
    diagnostics: &mut ProviderDiagnostics,
) -> Vec<ProviderInterval> {
    let future_limit = now.saturating_add(ACTIVE_GRACE_MILLIS);
    let mut intervals = facts
        .completed
        .iter()
        .filter_map(|interval| {
            if interval.start > future_limit || interval.end > future_limit {
                diagnostics.bad_events += 1;
                None
            } else {
                Some(interval.clone())
            }
        })
        .collect::<Vec<_>>();

    if let Some(open) = facts.open.as_ref() {
        let active = open.start < now
            && modified_at <= future_limit
            && now.saturating_sub(modified_at) <= ACTIVE_GRACE_MILLIS;
        if active {
            intervals.push(match open.provider {
                ProviderKind::Codex => codex_interval(
                    path,
                    open.start,
                    now,
                    "running",
                    "Codex 本机会话 task_started 进行中时间事件",
                ),
                ProviderKind::Claude => claude_interval(path, open.start, now, "running", 0.9),
            });
        } else if open.start > future_limit {
            diagnostics.bad_events += 1;
        }
    }
    intervals
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

fn claude_interval(
    path: &Path,
    start: u64,
    end: u64,
    status: &'static str,
    confidence: f64,
) -> ProviderInterval {
    let id = stable_id("claude", path, start);
    ProviderInterval {
        version: 1,
        start,
        end,
        provider: "claude",
        tool_id: "claude",
        tool_name: "Claude Code",
        agent_id: id.clone(),
        task_id: id,
        status,
        basis: if status == "running" {
            "Claude Code 本机会话 user 进行中时间事件"
        } else if confidence >= 0.95 {
            "Claude Code 本机会话 turn_duration 时间事件"
        } else {
            "Claude Code 本机会话 user/end_turn 时间事件"
        },
        confidence,
    }
}

fn record_io_error(diagnostics: &mut ProviderDiagnostics, error: &io::Error) {
    if error.kind() == io::ErrorKind::PermissionDenied {
        diagnostics.permission_failures += 1;
    } else {
        diagnostics.read_failures += 1;
    }
}

fn metadata_modified(metadata: &Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(system_millis)
        .unwrap_or(0)
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn stable_id(provider: &str, path: &Path, timestamp: u64) -> String {
    let mut digest = Sha256::new();
    digest.update(provider.as_bytes());
    digest.update(path.to_string_lossy().as_bytes());
    digest.update(timestamp.to_le_bytes());
    hex::encode(&digest.finalize()[..8])
}

pub(crate) fn unix_millis() -> u64 {
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
    use chrono::{SecondsFormat, Utc};

    fn fixture_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "itime-provider-{name}-{}-{}",
            std::process::id(),
            unix_millis()
        ))
    }

    fn write_provider_file(path: &Path, value: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, value).unwrap();
    }

    fn enabled_consent(codex: bool, claude: bool) -> ProviderConsent {
        ProviderConsent {
            notice_seen: true,
            codex_enabled: codex,
            claude_enabled: claude,
            ..ProviderConsent::default()
        }
    }

    #[test]
    fn unauthorized_snapshot_scans_zero_files_even_when_provider_files_exist() {
        let home = fixture_path("disabled");
        write_provider_file(
            &home.join(r".codex\sessions\rollout-private.jsonl"),
            "private",
        );
        let service = ProviderActivityService::default();
        let snapshot = service
            .snapshot_with_home(1, 2, 2, &home, ProviderConsent::default())
            .unwrap();
        let _ = fs::remove_dir_all(&home);

        assert_eq!(snapshot.status, "disabled");
        assert_eq!(snapshot.scanned_files, 0);
        assert_eq!(snapshot.diagnostics.candidate_files, 0);
        assert!(snapshot.intervals.is_empty());
        assert!(service
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty());
    }

    #[test]
    fn reads_codex_task_boundaries_without_content_fields() {
        let path = fixture_path("codex.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T01:00:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:03:00.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":"ignored"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:05:00.000Z","type":"event_msg","payload":{"type":"task_complete"}}"#,
                "\n"
            ),
        );
        let parsed = parse_codex_file(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(parsed.facts.completed.len(), 1);
        assert_eq!(
            parsed.facts.completed[0].end - parsed.facts.completed[0].start,
            5 * 60_000
        );
        let json = serde_json::to_string(&parsed.facts.completed[0]).unwrap();
        assert!(!json.contains("content"));
    }

    #[test]
    fn ignores_large_claude_content_without_storing_or_inspecting_it() {
        let path = fixture_path("claude-large.jsonl");
        let huge_content = "private".repeat(200_000);
        let line = format!(
            r#"{{"timestamp":"2026-07-18T02:00:00.000Z","type":"user","message":{{"role":"user","content":{{"unexpected":"{huge_content}"}}}}}}"#
        );
        write_provider_file(&path, &line);
        let parsed = parse_claude_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert!(parsed.facts.completed.is_empty());
        assert!(parsed.facts.open.is_some());
        assert_eq!(parsed.diagnostics.bad_lines, 0);
        assert!(!format!("{:?}", parsed.diagnostics).contains("private"));
    }

    #[test]
    fn running_interval_is_recalculated_when_cached_file_is_reused() {
        let home = fixture_path("running");
        let path = home.join(r".codex\sessions\rollout-running.jsonl");
        let now = unix_millis();
        let started = now - 60_000;
        let timestamp = DateTime::<Utc>::from_timestamp_millis(started as i64)
            .unwrap()
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        write_provider_file(
            &path,
            &format!(
                r#"{{"timestamp":"{timestamp}","type":"event_msg","payload":{{"type":"task_started"}}}}"#
            ),
        );
        let service = ProviderActivityService::new(enabled_consent(true, false));
        let first = service
            .snapshot_with_home(
                started - 1,
                now + 120_000,
                now,
                &home,
                enabled_consent(true, false),
            )
            .unwrap();
        let second = service
            .snapshot_with_home(
                started - 1,
                now + 120_000,
                now + 30_000,
                &home,
                enabled_consent(true, false),
            )
            .unwrap();
        let _ = fs::remove_dir_all(&home);

        assert_eq!(first.intervals[0].end, now);
        assert_eq!(second.intervals[0].end, now + 30_000);
        assert_eq!(second.diagnostics.cache_hits, 1);
    }

    #[test]
    fn deleted_files_are_evicted_from_cache_and_snapshot() {
        let home = fixture_path("deleted");
        let path = home.join(r".codex\sessions\rollout-deleted.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T01:00:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:05:00.000Z","type":"event_msg","payload":{"type":"task_complete"}}"#
            ),
        );
        let service = ProviderActivityService::new(enabled_consent(true, false));
        let start = parse_timestamp(Some("2026-07-18T00:00:00.000Z")).unwrap();
        let end = parse_timestamp(Some("2026-07-19T00:00:00.000Z")).unwrap();
        let now = parse_timestamp(Some("2026-07-19T01:00:00.000Z")).unwrap();
        let first = service
            .snapshot_with_home(start, end, now, &home, enabled_consent(true, false))
            .unwrap();
        fs::remove_file(&path).unwrap();
        let second = service
            .snapshot_with_home(start, end, now, &home, enabled_consent(true, false))
            .unwrap();
        let _ = fs::remove_dir_all(&home);

        assert_eq!(first.intervals.len(), 1);
        assert!(second.intervals.is_empty());
        assert!(service
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty());
    }

    #[test]
    fn replaced_files_invalidate_cached_facts() {
        let home = fixture_path("replaced");
        let path = home.join(r".codex\sessions\rollout-replaced.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T01:00:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:01:00.000Z","type":"event_msg","payload":{"type":"task_complete"}}"#
            ),
        );
        let service = ProviderActivityService::new(enabled_consent(true, false));
        let start = parse_timestamp(Some("2026-07-18T00:00:00.000Z")).unwrap();
        let end = parse_timestamp(Some("2026-07-19T00:00:00.000Z")).unwrap();
        let now = parse_timestamp(Some("2026-07-19T01:00:00.000Z")).unwrap();
        let first = service
            .snapshot_with_home(start, end, now, &home, enabled_consent(true, false))
            .unwrap();
        write_provider_file(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T03:00:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T03:07:00.000Z","type":"event_msg","payload":{"type":"task_complete"}}"#,
                "\n"
            ),
        );
        let second = service
            .snapshot_with_home(start, end, now, &home, enabled_consent(true, false))
            .unwrap();
        let _ = fs::remove_dir_all(&home);

        assert_eq!(first.intervals[0].end - first.intervals[0].start, 60_000);
        assert_eq!(
            second.intervals[0].end - second.intervals[0].start,
            7 * 60_000
        );
        assert_eq!(second.diagnostics.parsed_files, 1);
        assert_eq!(second.diagnostics.cache_hits, 0);
    }

    #[test]
    fn selects_newest_files_after_considering_more_than_4096_candidates() {
        let mut candidates = (0..5_000)
            .map(|index| Candidate {
                path: PathBuf::from(format!("{index}.jsonl")),
                kind: ProviderKind::Claude,
                modified_at: index,
                length: 1,
            })
            .collect::<Vec<_>>();
        select_newest_candidates(&mut candidates);

        assert_eq!(candidates.len(), MAX_PROVIDER_FILES);
        assert_eq!(candidates[0].modified_at, 4_999);
        assert_eq!(candidates.last().unwrap().modified_at, 2_952);
    }

    #[test]
    fn reports_mixed_bad_lines_and_bad_events_without_losing_valid_turns() {
        let path = fixture_path("mixed.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"type":"system","subtype":"turn_duration","broken":}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T02:10:00.000Z","type":"system","subtype":"turn_duration"}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T02:10:00.000Z","type":"system","subtype":"turn_duration","durationMs":120000}"#
            ),
        );
        let parsed = parse_claude_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(parsed.diagnostics.bad_lines, 1);
        assert_eq!(parsed.diagnostics.bad_events, 1);
        assert_eq!(parsed.facts.completed.len(), 1);
    }

    #[test]
    fn classifies_permission_failures_separately() {
        let mut diagnostics = ProviderDiagnostics::default();
        record_io_error(
            &mut diagnostics,
            &io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
        );
        assert_eq!(diagnostics.permission_failures, 1);
        assert_eq!(diagnostics.read_failures, 0);
    }

    #[test]
    fn excludes_future_intervals_and_future_running_starts() {
        let now = parse_timestamp(Some("2026-07-18T02:00:00.000Z")).unwrap();
        let future = now + ACTIVE_GRACE_MILLIS + 1;
        let path = PathBuf::from("future.jsonl");
        let facts = ParsedFileFacts {
            completed: vec![codex_interval(
                &path,
                future,
                future + 1_000,
                "completed",
                "future",
            )],
            open: Some(OpenInterval {
                start: future,
                provider: ProviderKind::Codex,
            }),
        };
        let mut diagnostics = ProviderDiagnostics::default();
        let intervals = materialize_facts(&path, &facts, now, now, &mut diagnostics);

        assert!(intervals.is_empty());
        assert_eq!(diagnostics.bad_events, 2);
    }

    #[test]
    fn clock_rollback_does_not_create_negative_work_and_cross_day_work_is_preserved() {
        let path = PathBuf::from("clock.jsonl");
        let day_start = parse_timestamp(Some("2026-07-18T00:00:00.000Z")).unwrap();
        let now = day_start + 30_000;
        let facts = ParsedFileFacts {
            completed: vec![codex_interval(
                &path,
                day_start - 60_000,
                day_start + 60_000,
                "completed",
                "cross-day",
            )],
            open: Some(OpenInterval {
                start: now + 1_000,
                provider: ProviderKind::Codex,
            }),
        };
        let mut diagnostics = ProviderDiagnostics::default();
        let intervals = materialize_facts(&path, &facts, now, now, &mut diagnostics);

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].end - intervals[0].start, 120_000);
        assert_eq!(intervals[0].status, "completed");
    }
}
