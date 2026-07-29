use crate::settings::{self, ProviderConsent};
use chrono::DateTime;
use rusqlite::{Connection, OpenFlags};
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

/// Coding agents aligned with Open Design / CC Switch style discovery.
/// Exact session parsers exist only for a subset; others are detect-only when installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProviderKind {
    Claude,
    Codex,
    OpenCode,
    GrokBuild,
    Copilot,
    Cursor,
    Antigravity,
    Hermes,
    OpenClaw,
    Gemini,
    Qwen,
    Kimi,
    Trae,
    Devin,
    Pi,
    Aider,
    DeepSeek,
    Kiro,
    Qoder,
    Vibe,
    Amp,
    Reasonix,
    Kilo,
    CodeBuddy,
    AtomCode,
}

impl ProviderKind {
    const ALL: [Self; 25] = [
        Self::Claude,
        Self::Codex,
        Self::OpenCode,
        Self::GrokBuild,
        Self::Copilot,
        Self::Cursor,
        Self::Antigravity,
        Self::Hermes,
        Self::OpenClaw,
        Self::Gemini,
        Self::Qwen,
        Self::Kimi,
        Self::Trae,
        Self::Devin,
        Self::Pi,
        Self::Aider,
        Self::DeepSeek,
        Self::Kiro,
        Self::Qoder,
        Self::Vibe,
        Self::Amp,
        Self::Reasonix,
        Self::Kilo,
        Self::CodeBuddy,
        Self::AtomCode,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude-code",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::GrokBuild => "grok-build",
            Self::Copilot => "copilot",
            Self::Cursor => "cursor",
            Self::Antigravity => "antigravity",
            Self::Hermes => "hermes",
            Self::OpenClaw => "openclaw",
            Self::Gemini => "gemini",
            Self::Qwen => "qwen",
            Self::Kimi => "kimi",
            Self::Trae => "trae",
            Self::Devin => "devin",
            Self::Pi => "pi",
            Self::Aider => "aider",
            Self::DeepSeek => "deepseek",
            Self::Kiro => "kiro",
            Self::Qoder => "qoder",
            Self::Vibe => "vibe",
            Self::Amp => "amp",
            Self::Reasonix => "reasonix",
            Self::Kilo => "kilo",
            Self::CodeBuddy => "codebuddy",
            Self::AtomCode => "atomcode",
        }
    }

    const fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
            Self::OpenCode => "OpenCode",
            Self::GrokBuild => "Grok Build",
            Self::Copilot => "GitHub Copilot CLI",
            Self::Cursor => "Cursor Agent",
            Self::Antigravity => "Antigravity",
            Self::Hermes => "Hermes",
            Self::OpenClaw => "OpenClaw",
            Self::Gemini => "Gemini CLI",
            Self::Qwen => "Qwen Code",
            Self::Kimi => "Kimi CLI",
            Self::Trae => "Trae CLI",
            Self::Devin => "Devin for Terminal",
            Self::Pi => "Pi",
            Self::Aider => "Aider",
            Self::DeepSeek => "DeepSeek TUI",
            Self::Kiro => "Kiro CLI",
            Self::Qoder => "Qoder CLI",
            Self::Vibe => "Mistral Vibe",
            Self::Amp => "Amp",
            Self::Reasonix => "Reasonix",
            Self::Kilo => "Kilo",
            Self::CodeBuddy => "CodeBuddy",
            Self::AtomCode => "AtomCode",
        }
    }

    /// PATH / launcher names (Open Design style).
    const fn binaries(self) -> &'static [&'static str] {
        match self {
            Self::Claude => &["claude"],
            Self::Codex => &["codex"],
            Self::OpenCode => &["opencode"],
            Self::GrokBuild => &["grok"],
            Self::Copilot => &["copilot"],
            Self::Cursor => &["cursor-agent", "cursor"],
            Self::Antigravity => &["antigravity"],
            Self::Hermes => &["hermes"],
            Self::OpenClaw => &["openclaw"],
            Self::Gemini => &["gemini"],
            Self::Qwen => &["qwen"],
            Self::Kimi => &["kimi"],
            Self::Trae => &["traecli", "trae"],
            Self::Devin => &["devin"],
            Self::Pi => &["pi"],
            Self::Aider => &["aider"],
            Self::DeepSeek => &["deepseek"],
            Self::Kiro => &["kiro"],
            Self::Qoder => &["qoder"],
            Self::Vibe => &["vibe"],
            Self::Amp => &["amp"],
            Self::Reasonix => &["reasonix"],
            Self::Kilo => &["kilo"],
            Self::CodeBuddy => &["codebuddy"],
            Self::AtomCode => &["atomcode"],
        }
    }

    const fn format_version(self) -> &'static str {
        match self {
            Self::Claude => "projects-jsonl-v1",
            Self::Codex => "rollout-jsonl-v1",
            Self::OpenCode => "sqlite-message-time-v1",
            Self::GrokBuild => "updates-jsonl-v1",
            Self::Copilot => "session-events-jsonl-v1",
            _ => "detect-only",
        }
    }

    const fn supports_exact_intervals(self) -> bool {
        matches!(
            self,
            Self::Codex | Self::Claude | Self::OpenCode | Self::GrokBuild | Self::Copilot
        )
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderInterval {
    pub(crate) version: u8,
    pub(crate) start: u64,
    pub(crate) end: u64,
    pub(crate) provider: &'static str,
    pub(crate) tool_id: &'static str,
    pub(crate) tool_name: &'static str,
    pub(crate) agent_id: String,
    pub(crate) task_id: String,
    pub(crate) status: &'static str,
    pub(crate) basis: &'static str,
    pub(crate) confidence: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderCapabilities {
    content_captured: bool,
    tools: Vec<AgentToolCapability>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentToolCapability {
    tool_id: &'static str,
    display_name: &'static str,
    installed: bool,
    format_version: &'static str,
    exact_task_count: bool,
    exact_duration: bool,
    exact_concurrency: bool,
    diagnostic_status: &'static str,
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

#[derive(Clone, Default)]
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
        if !consent.ai_agent_tools_enabled {
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
        if !consent.ai_agent_tools_enabled {
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
        if !consent.ai_agent_tools_enabled {
            return Ok(disabled_snapshot(consent));
        }

        let path_bins = discover_path_binaries();
        let cutoff = start.saturating_sub(2 * DAY_MILLIS);
        let mut diagnostics = ProviderDiagnostics::default();
        let mut candidates = Vec::new();
        let codex_available = collect_candidates(
            &provider_root(home, ProviderKind::Codex),
            ProviderKind::Codex,
            cutoff,
            6,
            &mut candidates,
            &mut diagnostics,
        );
        let claude_available = collect_candidates(
            &provider_root(home, ProviderKind::Claude),
            ProviderKind::Claude,
            cutoff,
            6,
            &mut candidates,
            &mut diagnostics,
        );
        let grok_available = collect_candidates(
            &provider_root(home, ProviderKind::GrokBuild),
            ProviderKind::GrokBuild,
            cutoff,
            6,
            &mut candidates,
            &mut diagnostics,
        );
        let opencode_available = collect_file_candidate(
            &provider_root(home, ProviderKind::OpenCode),
            ProviderKind::OpenCode,
            &mut candidates,
            &mut diagnostics,
        );
        let copilot_available = collect_candidates(
            &provider_root(home, ProviderKind::Copilot),
            ProviderKind::Copilot,
            cutoff,
            4,
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

        let tools = tool_capabilities(
            home,
            &path_bins,
            &[
                (ProviderKind::Codex, codex_available),
                (ProviderKind::Claude, claude_available),
                (ProviderKind::OpenCode, opencode_available),
                (ProviderKind::GrokBuild, grok_available),
                (ProviderKind::Copilot, copilot_available),
            ],
        );
        // Only installed agents affect aggregate status. Uninstalled registry
        // entries stay silent (CC Switch / Open Design style).
        let any_installed = tools.iter().any(|tool| tool.installed);
        let any_exact_installed = tools
            .iter()
            .any(|tool| tool.installed && tool.exact_duration);
        let status = if !any_installed {
            "unavailable"
        } else if any_exact_installed {
            if diagnostics.has_degradation() {
                "partial"
            } else {
                "ready"
            }
        } else {
            // Installed agents exist but none expose exact session intervals yet.
            "partial"
        };

        Ok(ProviderActivitySnapshot {
            source: "已授权的 AI Agent 编程工具本机会话结构元数据",
            status,
            updated_at: u128::from(now),
            scanned_files: candidates.len(),
            skipped_files,
            intervals,
            consent,
            diagnostics,
            capabilities: ProviderCapabilities {
                content_captured: false,
                tools,
            },
        })
    }

    fn load_file(&self, candidate: &Candidate) -> io::Result<(ParsedFile, bool)> {
        let metadata = fs::metadata(&candidate.path)?;
        let modified_at = metadata_modified(&metadata);
        if candidate.kind != ProviderKind::OpenCode {
            if let Some(cached) = self
                .cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(&candidate.path)
                .filter(|cached| {
                    cached.length == metadata.len() && cached.modified_at == modified_at
                })
                .cloned()
            {
                return Ok((cached.parsed, true));
            }
        }

        let parsed = match candidate.kind {
            ProviderKind::Codex => parse_codex_file(&candidate.path),
            ProviderKind::Claude => parse_claude_file(&candidate.path),
            ProviderKind::OpenCode => parse_opencode_db(&candidate.path),
            ProviderKind::GrokBuild => parse_grok_file(&candidate.path),
            ProviderKind::Copilot => parse_copilot_file(&candidate.path),
            _ => Ok(ParsedFile::default()),
        }?;
        if candidate.kind != ProviderKind::OpenCode {
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
        }
        Ok((parsed, false))
    }
}

fn disabled_snapshot(consent: ProviderConsent) -> ProviderActivitySnapshot {
    ProviderActivitySnapshot {
        source: "AI Agent 编程工具读取未授权",
        status: "disabled",
        updated_at: u128::from(unix_millis()),
        scanned_files: 0,
        skipped_files: 0,
        intervals: Vec::new(),
        consent,
        diagnostics: ProviderDiagnostics::default(),
        capabilities: ProviderCapabilities {
            content_captured: false,
            tools: ProviderKind::ALL
                .into_iter()
                .map(|kind| AgentToolCapability {
                    tool_id: kind.id(),
                    display_name: kind.display_name(),
                    installed: false,
                    format_version: kind.format_version(),
                    exact_task_count: false,
                    exact_duration: false,
                    exact_concurrency: false,
                    diagnostic_status: "disabled",
                })
                .collect(),
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
    telemetry: State<'_, crate::telemetry::TelemetryService>,
    consent: ProviderConsent,
) -> Result<ProviderConsent, String> {
    if !consent.ai_agent_tools_enabled {
        telemetry.set_enabled(false)?;
    }
    let saved = providers.set_consent(consent)?;
    if saved.ai_agent_tools_enabled {
        telemetry.set_enabled(true)?;
    }
    Ok(saved)
}

#[tauri::command]
pub(crate) fn get_provider_activity_snapshot(
    providers: State<'_, ProviderActivityService>,
    telemetry: State<'_, crate::telemetry::TelemetryService>,
    start: u64,
    end: u64,
) -> Result<ProviderActivitySnapshot, String> {
    let began = std::time::Instant::now();
    let snapshot = providers.snapshot(start, end);
    telemetry
        .performance()
        .record_agent_scan(began.elapsed(), snapshot.is_err());
    if let Ok(snapshot) = snapshot.as_ref() {
        telemetry.record_agent_intervals(&snapshot.intervals);
    }
    snapshot
}

fn provider_root(home: &Path, kind: ProviderKind) -> PathBuf {
    match kind {
        ProviderKind::Cursor => home.join(".cursor"),
        ProviderKind::Antigravity => home.join(r".gemini\antigravity-cli"),
        ProviderKind::Codex => home.join(r".codex\sessions"),
        ProviderKind::Claude => home.join(r".claude\projects"),
        ProviderKind::OpenCode => home.join(r".local\share\opencode\opencode.db"),
        ProviderKind::GrokBuild => home.join(r".grok\sessions"),
        ProviderKind::Hermes => home.join(".hermes"),
        ProviderKind::OpenClaw => home.join(".openclaw"),
        ProviderKind::Copilot => home.join(r".copilot\session-state"),
        ProviderKind::Gemini => home.join(".gemini"),
        ProviderKind::Qwen => home.join(".qwen"),
        ProviderKind::Kimi => home.join(".kimi"),
        ProviderKind::Trae => home.join(".trae"),
        ProviderKind::Devin => home.join(".devin"),
        ProviderKind::Pi => home.join(".pi"),
        ProviderKind::Aider => home.join(".aider"),
        ProviderKind::DeepSeek => home.join(".deepseek"),
        ProviderKind::Kiro => home.join(".kiro"),
        ProviderKind::Qoder => home.join(".qoder"),
        ProviderKind::Vibe => home.join(".vibe"),
        ProviderKind::Amp => home.join(".amp"),
        ProviderKind::Reasonix => home.join(".reasonix"),
        ProviderKind::Kilo => home.join(".kilo"),
        ProviderKind::CodeBuddy => home.join(".codebuddy"),
        ProviderKind::AtomCode => home.join(".atomcode"),
    }
}

fn discover_path_binaries() -> HashSet<String> {
    let mut found = HashSet::new();
    let Some(path) = std::env::var_os("PATH") else {
        return found;
    };
    let names: Vec<&str> = ProviderKind::ALL
        .into_iter()
        .flat_map(ProviderKind::binaries)
        .copied()
        .collect();
    for dir in std::env::split_paths(&path) {
        for name in &names {
            for ext in ["", ".exe", ".cmd", ".bat", ".ps1"] {
                let candidate = dir.join(format!("{name}{ext}"));
                if candidate.is_file() {
                    found.insert((*name).to_ascii_lowercase());
                    break;
                }
            }
        }
    }
    found
}

fn has_path_binary(path_bins: &HashSet<String>, kind: ProviderKind) -> bool {
    kind.binaries()
        .iter()
        .any(|name| path_bins.contains(&name.to_ascii_lowercase()))
}

/// Prefer PATH (like CC Switch / Open Design). Fall back only when real session
/// evidence exists — empty leftover config folders do not count as installed.
fn is_agent_installed(home: &Path, path_bins: &HashSet<String>, kind: ProviderKind) -> bool {
    if has_path_binary(path_bins, kind) {
        return true;
    }
    has_session_evidence(home, kind)
}

fn has_session_evidence(home: &Path, kind: ProviderKind) -> bool {
    match kind {
        ProviderKind::Codex => directory_has_matching_file(&provider_root(home, kind), 6, |path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
        }),
        ProviderKind::Claude => {
            directory_has_matching_file(&provider_root(home, kind), 6, |path| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("jsonl"))
            })
        }
        ProviderKind::GrokBuild => {
            directory_has_matching_file(&provider_root(home, kind), 6, |path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("updates.jsonl"))
            })
        }
        ProviderKind::OpenCode => provider_root(home, kind).is_file(),
        ProviderKind::Copilot => {
            directory_has_matching_file(&provider_root(home, kind), 4, |path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("events.jsonl"))
            })
        }
        ProviderKind::Cursor => {
            directory_has_matching_file(&home.join(".cursor").join("projects"), 6, |path| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("jsonl"))
            })
        }
        ProviderKind::Antigravity => {
            directory_has_matching_file(&provider_root(home, kind), 5, |_| true)
                || home.join(r"AppData\Roaming\Antigravity").exists()
        }
        ProviderKind::Hermes => {
            home.join(".hermes").exists() || home.join(".hermes-agent").exists()
        }
        // Empty leftover folders (e.g. .openclaw) must not mark installed.
        _ => false,
    }
}

fn directory_has_matching_file(
    root: &Path,
    depth: u8,
    predicate: impl Fn(&Path) -> bool + Copy,
) -> bool {
    if depth == 0 || !root.is_dir() {
        return false;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if directory_has_matching_file(&path, depth - 1, predicate) {
                return true;
            }
        } else if predicate(&path) {
            return true;
        }
    }
    false
}

fn tool_capabilities(
    home: &Path,
    path_bins: &HashSet<String>,
    exact_availability: &[(ProviderKind, bool)],
) -> Vec<AgentToolCapability> {
    ProviderKind::ALL
        .into_iter()
        .map(|kind| {
            let installed = is_agent_installed(home, path_bins, kind);
            let exact_available = exact_availability
                .iter()
                .find_map(|(candidate, available)| (*candidate == kind).then_some(*available))
                .unwrap_or(false)
                && installed;
            let diagnostic_status = if !installed {
                "notInstalled"
            } else if exact_available {
                "ready"
            } else if kind.supports_exact_intervals() {
                "schemaChanged"
            } else {
                "detectedUnsupported"
            };
            AgentToolCapability {
                tool_id: kind.id(),
                display_name: kind.display_name(),
                installed,
                format_version: kind.format_version(),
                exact_task_count: exact_available,
                exact_duration: exact_available,
                exact_concurrency: exact_available,
                diagnostic_status,
            }
        })
        .collect()
}

fn collect_file_candidate(
    path: &Path,
    kind: ProviderKind,
    output: &mut Vec<Candidate>,
    diagnostics: &mut ProviderDiagnostics,
) -> bool {
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => return false,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return false,
        Err(error) => {
            record_io_error(diagnostics, &error);
            return false;
        }
    };
    output.push(Candidate {
        path: path.to_path_buf(),
        kind,
        modified_at: metadata_modified(&metadata),
        length: metadata.len(),
    });
    true
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
        ProviderKind::GrokBuild => path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("updates.jsonl")),
        ProviderKind::Copilot => path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("events.jsonl")),
        ProviderKind::OpenCode => false,
        _ => false,
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
        // Only consider root-level event_msg rows. Tool output often embeds the
        // substrings task_started/task_complete/turn_aborted in source code or
        // logs; those must not become bad_events or force full JSON parsing.
        if !looks_like_codex_lifecycle_line(&line) {
            line.clear();
            continue;
        }
        match serde_json::from_slice::<WireEvent>(&line) {
            Ok(event) => {
                // Lines that only *contain* lifecycle tokens (e.g. patch_apply_end
                // payloads quoting source) are unrelated noise — skip silently.
                let Some((kind, timestamp)) = codex_event(&event) else {
                    line.clear();
                    continue;
                };
                match kind {
                    "task_started" => {
                        // Codex occasionally emits near-duplicate starts a few
                        // milliseconds apart. Keep the latest start; do not treat
                        // that as an anomalous event.
                        open_start = Some(timestamp);
                    }
                    "task_complete" | "turn_aborted" => {
                        if let Some(start) = open_start.take().filter(|start| timestamp > *start) {
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
                        }
                        // Orphan complete/abort (resume, crash recovery, etc.) is
                        // incomplete evidence, not a parse anomaly.
                    }
                    _ => {}
                }
            }
            // Only count as a bad line when the root object looked like a lifecycle
            // event_msg but the JSON itself was unreadable.
            Err(_) => diagnostics.bad_lines += 1,
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
                    // User rows without a usable timestamp are incomplete, not anomalous.
                    if let Some(timestamp) = timestamp {
                        latest_human_start = Some(timestamp);
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
                    }
                } else if event.record_type.as_deref() == Some("system")
                    && event.subtype.as_deref() == Some("turn_duration")
                {
                    // turn_duration is the primary interval signal; malformed
                    // records here are real anomalies.
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
                }
                // Other prefilter hits (content mentioning end_turn / type user,
                // side-channel system rows, etc.) are unrelated noise — ignore.
            }
            // Prefilter is broad (content may mention end_turn). Only count a bad
            // line when the row itself looks like a turn_duration system event.
            Err(_) => {
                if looks_like_claude_turn_duration_line(&line) {
                    diagnostics.bad_lines += 1;
                }
            }
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

fn parse_opencode_db(path: &Path) -> io::Result<ParsedFile> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_io_error)?;
    connection
        .busy_timeout(std::time::Duration::from_millis(750))
        .map_err(sqlite_io_error)?;
    let mut statement = connection
        .prepare(
            "SELECT time_created, json_extract(data, '$.time.completed') \
             FROM message \
             WHERE json_extract(data, '$.role') = 'assistant' \
               AND json_extract(data, '$.time.completed') IS NOT NULL",
        )
        .map_err(sqlite_io_error)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?)))
        .map_err(sqlite_io_error)?;
    let mut facts = ParsedFileFacts::default();
    let mut diagnostics = ParseDiagnostics::default();
    for row in rows {
        match row {
            Ok((start, end)) if end > start && end - start <= 7 * DAY_MILLIS => {
                facts.completed.push(opencode_interval(path, start, end));
            }
            Ok(_) => diagnostics.bad_events += 1,
            Err(_) => diagnostics.bad_lines += 1,
        }
    }
    Ok(ParsedFile { facts, diagnostics })
}

fn sqlite_io_error(error: rusqlite::Error) -> io::Error {
    let kind = match &error {
        rusqlite::Error::SqliteFailure(code, _)
            if code.code == rusqlite::ErrorCode::ReadOnly
                || code.code == rusqlite::ErrorCode::PermissionDenied =>
        {
            io::ErrorKind::PermissionDenied
        }
        _ => io::ErrorKind::InvalidData,
    };
    io::Error::new(kind, error.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrokUpdateLine {
    params: Option<GrokParams>,
}

#[derive(Deserialize)]
struct GrokParams {
    update: Option<GrokUpdate>,
    #[serde(rename = "_meta")]
    metadata: Option<GrokMetadata>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrokUpdate {
    session_update: Option<String>,
    #[serde(alias = "prompt_id")]
    prompt_id: Option<String>,
    #[serde(rename = "_meta")]
    metadata: Option<GrokMetadata>,
}

/// Shared by `params._meta` and `update._meta`. Current Grok Build puts
/// `turnStartMs` / `promptId` / `agentTimestampMs` on **params._meta**.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrokMetadata {
    #[serde(alias = "prompt_id")]
    prompt_id: Option<String>,
    #[serde(alias = "turn_start_ms")]
    turn_start_ms: Option<u64>,
    #[serde(alias = "agent_timestamp_ms")]
    agent_timestamp_ms: Option<u64>,
}

fn looks_like_grok_turn_line(line: &[u8]) -> bool {
    contains_bytes(line, b"turn_completed")
        || contains_bytes(line, b"\"turnStartMs\"")
        || contains_bytes(line, b"\"turn_start_ms\"")
}

fn parse_grok_file(path: &Path) -> io::Result<ParsedFile> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = Vec::new();
    let mut starts = HashMap::<String, u64>::new();
    let mut facts = ParsedFileFacts::default();
    let mut diagnostics = ParseDiagnostics::default();
    while reader.read_until(b'\n', &mut line)? > 0 {
        // Prefer the live Grok Build markers; keep snake_case for older files.
        let relevant = contains_bytes(&line, b"turnStartMs")
            || contains_bytes(&line, b"turn_start_ms")
            || contains_bytes(&line, b"turn_completed");
        if !relevant {
            line.clear();
            continue;
        }
        match serde_json::from_slice::<GrokUpdateLine>(&line) {
            Ok(event) => {
                let Some(params) = event.params else {
                    line.clear();
                    continue;
                };
                let params_meta = params.metadata.as_ref();
                let update = params.update.as_ref();
                let update_meta = update.and_then(|item| item.metadata.as_ref());

                // Current Grok Build stores prompt/turn clocks on params._meta;
                // fall back to update fields for older session layouts.
                let prompt_id = params_meta
                    .and_then(|meta| meta.prompt_id.clone())
                    .or_else(|| update_meta.and_then(|meta| meta.prompt_id.clone()))
                    .or_else(|| update.and_then(|item| item.prompt_id.clone()));
                let turn_start = params_meta
                    .and_then(|meta| meta.turn_start_ms)
                    .or_else(|| update_meta.and_then(|meta| meta.turn_start_ms));

                if let (Some(prompt_id), Some(start)) = (prompt_id.clone(), turn_start) {
                    starts.entry(prompt_id).or_insert(start);
                }

                let completed = update.and_then(|item| item.session_update.as_deref())
                    == Some("turn_completed");
                if completed {
                    let end = params_meta
                        .and_then(|meta| meta.agent_timestamp_ms)
                        .or_else(|| update_meta.and_then(|meta| meta.agent_timestamp_ms));
                    match (prompt_id.and_then(|id| starts.remove(&id)), end) {
                        (Some(start), Some(end))
                            if end > start && end - start <= 7 * DAY_MILLIS =>
                        {
                            facts.completed.push(grok_interval(path, start, end));
                        }
                        // Orphan / incomplete pairs (resume, truncated files, mid-session
                        // open) are incomplete evidence, not parse anomalies.
                        _ => {}
                    }
                }
            }
            // Broad prefilter can hit tool payloads; only count broken JSON that
            // still looks like a real turn boundary row.
            Err(_) if looks_like_grok_turn_line(&line) => diagnostics.bad_lines += 1,
            Err(_) => {}
        }
        line.clear();
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
                ProviderKind::Copilot => copilot_interval(path, open.start, now, "running"),
                _ => return intervals,
            });
        } else if open.start > future_limit {
            diagnostics.bad_events += 1;
        }
    }
    intervals
}

fn opencode_interval(path: &Path, start: u64, end: u64) -> ProviderInterval {
    let id = stable_id("opencode", path, start);
    ProviderInterval {
        version: 1,
        start,
        end,
        provider: "opencode",
        tool_id: "opencode",
        tool_name: "OpenCode",
        agent_id: id.clone(),
        task_id: id,
        status: "completed",
        basis: "OpenCode 本地 SQLite assistant time.created/time.completed 时间事件",
        confidence: 0.99,
    }
}

fn copilot_interval(path: &Path, start: u64, end: u64, status: &'static str) -> ProviderInterval {
    let id = stable_id("copilot", path, start);
    ProviderInterval {
        version: 1,
        start,
        end,
        provider: "copilot",
        tool_id: "copilot",
        tool_name: "GitHub Copilot CLI",
        agent_id: id.clone(),
        task_id: id,
        status,
        basis: "Copilot CLI 本机会话 assistant.turn_start/turn_end 时间事件",
        confidence: 0.98,
    }
}

fn parse_copilot_file(path: &Path) -> io::Result<ParsedFile> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = Vec::new();
    let mut open_start = None;
    let mut facts = ParsedFileFacts::default();
    let mut diagnostics = ParseDiagnostics::default();
    while reader.read_until(b'\n', &mut line)? > 0 {
        let relevant = contains_bytes(&line, b"assistant.turn_start")
            || contains_bytes(&line, b"assistant.turn_end")
            || contains_bytes(&line, b"session.shutdown");
        if !relevant {
            line.clear();
            continue;
        }
        match serde_json::from_slice::<CopilotEvent>(&line) {
            Ok(event) => {
                let Some(timestamp) = parse_timestamp(event.timestamp.as_deref()) else {
                    line.clear();
                    continue;
                };
                match event.record_type.as_deref() {
                    Some("assistant.turn_start") => open_start = Some(timestamp),
                    Some("assistant.turn_end") | Some("session.shutdown") => {
                        if let Some(start) = open_start.take().filter(|start| timestamp > *start) {
                            if timestamp - start <= 7 * DAY_MILLIS {
                                facts.completed.push(copilot_interval(
                                    path,
                                    start,
                                    timestamp,
                                    "completed",
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(_) => {
                if looks_like_copilot_turn_line(&line) {
                    diagnostics.bad_lines += 1;
                }
            }
        }
        line.clear();
    }
    if let Some(start) = open_start {
        facts.open = Some(OpenInterval {
            start,
            provider: ProviderKind::Copilot,
        });
    }
    Ok(ParsedFile { facts, diagnostics })
}

fn looks_like_copilot_turn_line(line: &[u8]) -> bool {
    contains_bytes(line, b"assistant.turn_start")
        || contains_bytes(line, b"assistant.turn_end")
        || contains_bytes(line, b"session.shutdown")
}

#[derive(Deserialize)]
struct CopilotEvent {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    record_type: Option<String>,
}

fn grok_interval(path: &Path, start: u64, end: u64) -> ProviderInterval {
    let id = stable_id("grok-build", path, start);
    ProviderInterval {
        version: 1,
        start,
        end,
        provider: "grok-build",
        tool_id: "grok-build",
        tool_name: "Grok Build",
        agent_id: id.clone(),
        task_id: id,
        status: "completed",
        basis: "Grok Build 本机会话 turnStartMs/turn_completed 时间事件",
        confidence: 0.99,
    }
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
    let id = stable_id("claude-code", path, start);
    ProviderInterval {
        version: 1,
        start,
        end,
        provider: "claude-code",
        tool_id: "claude-code",
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

/// Codex rollout lines place the root `"type"` near the start of the JSON object.
/// Restrict the prefilter to that prefix so tool-output bodies that quote
/// `task_started` / `event_msg` from source code are not treated as lifecycle rows.
fn looks_like_codex_lifecycle_line(line: &[u8]) -> bool {
    let head = line_head(line, 192);
    let has_event_msg = contains_bytes(head, br#""type":"event_msg""#)
        || contains_bytes(head, br#""type": "event_msg""#);
    if !has_event_msg {
        return false;
    }
    contains_bytes(line, br#""type":"task_started""#)
        || contains_bytes(line, br#""type": "task_started""#)
        || contains_bytes(line, br#""type":"task_complete""#)
        || contains_bytes(line, br#""type": "task_complete""#)
        || contains_bytes(line, br#""type":"turn_aborted""#)
        || contains_bytes(line, br#""type": "turn_aborted""#)
}

fn looks_like_claude_turn_duration_line(line: &[u8]) -> bool {
    let head = line_head(line, 256);
    (contains_bytes(head, br#""type":"system""#) || contains_bytes(head, br#""type": "system""#))
        && (contains_bytes(line, br#""subtype":"turn_duration""#)
            || contains_bytes(line, br#""subtype": "turn_duration""#))
}

fn line_head(line: &[u8], max_len: usize) -> &[u8] {
    &line[..line.len().min(max_len)]
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
            ai_agent_tools_enabled: codex || claude,
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
    fn reads_opencode_completed_assistant_times_without_message_content() {
        let path = fixture_path("opencode.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE message (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    time_created INTEGER NOT NULL,
                    time_updated INTEGER NOT NULL,
                    data TEXT NOT NULL
                );
                INSERT INTO message VALUES (
                    'private-message-id',
                    'private-session-id',
                    1752800000000,
                    1752800120000,
                    '{\"role\":\"assistant\",\"time\":{\"created\":1752800000000,\"completed\":1752800120000},\"content\":\"must-not-escape\"}'
                );",
            )
            .unwrap();
        drop(connection);

        let parsed = parse_opencode_db(&path).unwrap();
        let _ = fs::remove_file(path);
        assert_eq!(parsed.facts.completed.len(), 1);
        let json = serde_json::to_string(&parsed.facts.completed[0]).unwrap();
        assert!(!json.contains("must-not-escape"));
        assert!(!json.contains("private-message-id"));
        assert!(!json.contains("private-session-id"));
    }

    #[test]
    fn reads_grok_turn_boundaries_without_prompt_or_response_content() {
        let path = fixture_path("updates.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"method":"session/update","params":{"update":{"sessionUpdate":"agent_thought_chunk","content":{"text":"private prompt and response"},"_meta":{"promptId":"safe-prompt-key","turnStartMs":1752800000000}}}}"#,
                "\n",
                r#"{"method":"_x.ai/session/update","params":{"update":{"sessionUpdate":"turn_completed","prompt_id":"safe-prompt-key"},"_meta":{"agentTimestampMs":1752800120000}}}"#,
                "\n"
            ),
        );
        let parsed = parse_grok_file(&path).unwrap();
        let _ = fs::remove_file(path);

        assert_eq!(parsed.facts.completed.len(), 1);
        let json = serde_json::to_string(&parsed.facts.completed[0]).unwrap();
        assert!(!json.contains("private prompt"));
        assert!(!json.contains("safe-prompt-key"));
    }

    #[test]
    fn registry_covers_open_design_style_agents() {
        let ids = ProviderKind::ALL.map(ProviderKind::id);
        assert!(ids.contains(&"claude-code"));
        assert!(ids.contains(&"codex"));
        assert!(ids.contains(&"opencode"));
        assert!(ids.contains(&"grok-build"));
        assert!(ids.contains(&"copilot"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"qwen"));
        assert!(ids.contains(&"hermes"));
        assert_eq!(ProviderKind::ALL.len(), 25);
    }

    #[test]
    fn uninstalled_agents_do_not_count_as_installed_from_empty_folders() {
        let home = fixture_path("empty-openclaw");
        fs::create_dir_all(home.join(".openclaw")).unwrap();
        let path_bins = HashSet::new();
        assert!(!is_agent_installed(
            &home,
            &path_bins,
            ProviderKind::OpenClaw
        ));
        assert!(!is_agent_installed(&home, &path_bins, ProviderKind::Hermes));
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn copilot_turn_start_end_forms_interval() {
        let path = fixture_path("copilot-events.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"type":"assistant.turn_start","timestamp":"2026-07-18T01:00:00.000Z"}"#,
                "\n",
                r#"{"type":"assistant.message","timestamp":"2026-07-18T01:01:00.000Z"}"#,
                "\n",
                r#"{"type":"assistant.turn_end","timestamp":"2026-07-18T01:05:00.000Z"}"#,
                "\n"
            ),
        );
        let parsed = parse_copilot_file(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(parsed.facts.completed.len(), 1);
        assert_eq!(
            parsed.facts.completed[0].end - parsed.facts.completed[0].start,
            5 * 60_000
        );
        assert_eq!(parsed.diagnostics.bad_events, 0);
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
    fn codex_tool_output_mentioning_lifecycle_tokens_is_not_anomalous() {
        let path = fixture_path("codex-noise.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T01:00:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                // Tool output / source snippets commonly embed these tokens when
                // working on iTime itself — they must not inflate bad_events.
                r#"{"timestamp":"2026-07-18T01:01:00.000Z","type":"response_item","payload":{"type":"function_call_output","output":"contains_bytes(&line, b\"task_started\") task_complete turn_aborted \"type\":\"event_msg\""}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:02:00.000Z","type":"event_msg","payload":{"type":"patch_apply_end","changes":"is_task_complete"}}"#,
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
        assert_eq!(parsed.diagnostics.bad_lines, 0);
        assert_eq!(parsed.diagnostics.bad_events, 0);
    }

    #[test]
    fn codex_duplicate_task_started_keeps_latest_start_without_bad_events() {
        let path = fixture_path("codex-dup-start.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T01:00:00.000Z","type":"event_msg","payload":{"type":"task_started"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T01:00:00.050Z","type":"event_msg","payload":{"type":"task_started"}}"#,
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
            5 * 60_000 - 50
        );
        assert_eq!(parsed.diagnostics.bad_events, 0);
    }

    #[test]
    fn claude_unrelated_prefilter_hits_are_not_anomalous() {
        let path = fixture_path("claude-noise.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"timestamp":"2026-07-18T02:00:00.000Z","type":"assistant","message":{"role":"assistant","content":"mentions end_turn in prose"}}"#,
                "\n",
                r#"{"timestamp":"2026-07-18T02:10:00.000Z","type":"system","subtype":"turn_duration","durationMs":120000}"#,
                "\n"
            ),
        );
        let parsed = parse_claude_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(parsed.facts.completed.len(), 1);
        assert_eq!(parsed.diagnostics.bad_events, 0);
        assert_eq!(parsed.diagnostics.bad_lines, 0);
    }

    #[test]
    fn grok_reads_turn_clocks_from_params_meta() {
        // Live Grok Build puts turnStartMs / promptId / agentTimestampMs on params._meta.
        let path = fixture_path("grok-params-meta.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk"},"_meta":{"promptId":"p1","turnStartMs":1000,"agentTimestampMs":1500}}}"#,
                "\n",
                r#"{"method":"session/update","params":{"update":{"sessionUpdate":"turn_completed","prompt_id":"p1"},"_meta":{"agentTimestampMs":61000}}}"#,
                "\n",
                // Orphan complete without a matching start — incomplete, not anomalous.
                r#"{"method":"session/update","params":{"update":{"sessionUpdate":"turn_completed","prompt_id":"p-missing"},"_meta":{"agentTimestampMs":90000}}}"#,
                "\n"
            ),
        );
        let parsed = parse_grok_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(parsed.facts.completed.len(), 1);
        assert_eq!(
            parsed.facts.completed[0].end - parsed.facts.completed[0].start,
            60_000
        );
        assert_eq!(parsed.diagnostics.bad_events, 0);
        assert_eq!(parsed.diagnostics.bad_lines, 0);
    }

    #[test]
    fn grok_legacy_update_meta_layout_still_parses() {
        let path = fixture_path("grok-update-meta.jsonl");
        write_provider_file(
            &path,
            concat!(
                r#"{"params":{"update":{"sessionUpdate":"agent_message_chunk","_meta":{"promptId":"legacy","turnStartMs":2000}},"_meta":{"agentTimestampMs":2500}}}"#,
                "\n",
                r#"{"params":{"update":{"sessionUpdate":"turn_completed","promptId":"legacy","_meta":{}},"_meta":{"agentTimestampMs":32000}}}"#,
                "\n"
            ),
        );
        let parsed = parse_grok_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(parsed.facts.completed.len(), 1);
        assert_eq!(
            parsed.facts.completed[0].end - parsed.facts.completed[0].start,
            30_000
        );
        assert_eq!(parsed.diagnostics.bad_events, 0);
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
