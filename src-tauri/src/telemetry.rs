use crate::{atomic_json, provider_activity::ProviderInterval};
use chrono::{Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::State;
use uuid::Uuid;

const HEARTBEAT_INTERVAL_MILLIS: u64 = 6 * 60 * 60 * 1_000;
const MAX_OUTBOX_ITEMS: usize = 128;
const MAX_OUTBOX_BYTES: usize = 2 * 1024 * 1024;
const OUTBOX_MAX_AGE_MILLIS: u64 = 14 * 24 * 60 * 60 * 1_000;
const MAX_METRIC_SAMPLES: usize = 512;

#[derive(Clone, Debug, Default)]
pub(crate) struct PerformanceRecorder {
    activity_loop_millis: Arc<Mutex<Vec<u64>>>,
    agent_scan_millis: Arc<Mutex<Vec<u64>>>,
    agent_scan_errors: Arc<AtomicU64>,
    startup_to_ready_millis: Arc<AtomicU64>,
}

impl PerformanceRecorder {
    pub(crate) fn record_activity_loop(&self, elapsed: Duration) {
        push_sample(&self.activity_loop_millis, elapsed);
    }

    pub(crate) fn record_agent_scan(&self, elapsed: Duration, failed: bool) {
        push_sample(&self.agent_scan_millis, elapsed);
        if failed {
            self.agent_scan_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(crate) fn mark_ui_ready(&self, elapsed: Duration) {
        let millis = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let _ = self.startup_to_ready_millis.compare_exchange(
            0,
            millis.max(1),
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    fn snapshot(&self) -> PerformanceDaily {
        PerformanceDaily {
            date: local_date(crate::provider_activity::unix_millis()),
            startup_to_ready_millis: nonzero(self.startup_to_ready_millis.load(Ordering::Acquire)),
            activity_loop_p95_millis: p95(&self.activity_loop_millis),
            agent_scan_p95_millis: p95(&self.agent_scan_millis),
            agent_scan_errors: self.agent_scan_errors.load(Ordering::Acquire),
            peak_working_set_bytes: peak_working_set_bytes(),
        }
    }
}

fn push_sample(samples: &Mutex<Vec<u64>>, elapsed: Duration) {
    let millis = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    let mut samples = samples
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if samples.len() == MAX_METRIC_SAMPLES {
        samples.remove(0);
    }
    samples.push(millis);
}

fn p95(samples: &Mutex<Vec<u64>>) -> Option<u64> {
    let mut values = samples
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let index = ((values.len() * 95).div_ceil(100)).saturating_sub(1);
    values.get(index).copied()
}

fn nonzero(value: u64) -> Option<u64> {
    (value > 0).then_some(value)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryIdentity {
    version: u8,
    installation_id: String,
    #[serde(default)]
    hardware_hash: Option<String>,
    #[serde(default)]
    last_heartbeat_at: u64,
}

impl TelemetryIdentity {
    fn create() -> Self {
        Self {
            version: 1,
            installation_id: Uuid::new_v4().to_string(),
            hardware_hash: None,
            last_heartbeat_at: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryEnvelopeV1 {
    schema_version: u8,
    event_id: String,
    created_at: u64,
    installation_id: String,
    app_version: String,
    hardware: Option<HardwareSnapshot>,
    performance_daily: Option<PerformanceDaily>,
    agent_tool_daily: Vec<AgentToolDaily>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HardwareSnapshot {
    manufacturer: Option<String>,
    model: Option<String>,
    form_factor: String,
    cpu: CpuSnapshot,
    memory_bytes: Option<u64>,
    gpus: Vec<GpuSnapshot>,
    windows_version: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CpuSnapshot {
    name: Option<String>,
    vendor: Option<String>,
    architecture: Option<String>,
    physical_cores: Option<u32>,
    logical_threads: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GpuSnapshot {
    name: Option<String>,
    vendor: Option<String>,
    memory_bytes: Option<u64>,
    driver_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PerformanceDaily {
    date: String,
    startup_to_ready_millis: Option<u64>,
    activity_loop_p95_millis: Option<u64>,
    agent_scan_p95_millis: Option<u64>,
    agent_scan_errors: u64,
    peak_working_set_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentToolDaily {
    date: String,
    tool_id: String,
    task_count: usize,
    total_execution_seconds: u64,
    max_concurrency: usize,
}

#[derive(Default)]
struct TelemetryState {
    identity: Option<TelemetryIdentity>,
    outbox: Vec<TelemetryEnvelopeV1>,
    upload_failures: u8,
    next_retry_at: u64,
}

#[derive(Clone)]
pub(crate) struct TelemetryService {
    enabled: Arc<AtomicBool>,
    worker_started: Arc<AtomicBool>,
    state: Arc<Mutex<TelemetryState>>,
    performance: PerformanceRecorder,
    launched_at: Instant,
    identity_path: PathBuf,
    outbox_path: PathBuf,
    endpoint: Option<&'static str>,
}

impl TelemetryService {
    pub(crate) fn new(enabled: bool) -> Self {
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let root = local.join("iTime");
        let service = Self {
            enabled: Arc::new(AtomicBool::new(enabled)),
            worker_started: Arc::new(AtomicBool::new(false)),
            state: Arc::new(Mutex::new(TelemetryState::default())),
            performance: PerformanceRecorder::default(),
            launched_at: Instant::now(),
            identity_path: root.join(r"Config\telemetry-installation.json"),
            outbox_path: root.join(r"Data\Telemetry\outbox.json"),
            endpoint: option_env!("ITIME_TELEMETRY_ENDPOINT"),
        };
        if enabled {
            service.load_local_state();
        }
        service
    }

    pub(crate) fn performance(&self) -> PerformanceRecorder {
        self.performance.clone()
    }

    pub(crate) fn start(&self) {
        if self.worker_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let service = self.clone();
        let _ = thread::Builder::new()
            .name("itime-telemetry".into())
            .spawn(move || {
                thread::sleep(Duration::from_secs(30));
                loop {
                    if service.enabled.load(Ordering::Acquire) {
                        service.enqueue_heartbeat_if_due();
                        service.flush_outbox();
                    }
                    thread::sleep(Duration::from_secs(60));
                }
            });
    }

    pub(crate) fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        self.enabled.store(enabled, Ordering::Release);
        if enabled {
            self.load_local_state();
            self.performance.mark_ui_ready(self.launched_at.elapsed());
            self.enqueue_heartbeat_if_due();
            self.flush_outbox();
        } else {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.outbox.clear();
            state.upload_failures = 0;
            state.next_retry_at = 0;
            drop(state);
            if self.outbox_path.exists() {
                fs::remove_file(&self.outbox_path).map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    pub(crate) fn mark_ui_ready(&self) {
        if self.enabled.load(Ordering::Acquire) {
            self.performance.mark_ui_ready(self.launched_at.elapsed());
        }
    }

    pub(crate) fn prepare_for_update(&self) -> Result<(), String> {
        if self.enabled.load(Ordering::Acquire) {
            self.enqueue_heartbeat_if_due();
        }
        self.persist_outbox()
    }

    pub(crate) fn record_agent_intervals(&self, intervals: &[ProviderInterval]) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        let aggregates = aggregate_agent_days(intervals);
        if aggregates.is_empty() {
            return;
        }
        let mut by_date = BTreeMap::<String, Vec<AgentToolDaily>>::new();
        for aggregate in aggregates {
            by_date
                .entry(aggregate.date.clone())
                .or_default()
                .push(aggregate);
        }
        for (date, values) in by_date {
            if let Some(identity) = self.ensure_identity() {
                self.enqueue(TelemetryEnvelopeV1 {
                    schema_version: 1,
                    event_id: format!("agent-tool-daily:{date}"),
                    created_at: crate::provider_activity::unix_millis(),
                    installation_id: identity.installation_id,
                    app_version: env!("CARGO_PKG_VERSION").to_string(),
                    hardware: None,
                    performance_daily: None,
                    agent_tool_daily: values,
                });
            }
        }
        self.flush_outbox();
    }

    fn load_local_state(&self) {
        let identity = read_json::<TelemetryIdentity>(&self.identity_path)
            .filter(|identity| identity.version == 1)
            .unwrap_or_else(TelemetryIdentity::create);
        let outbox = read_json::<Vec<TelemetryEnvelopeV1>>(&self.outbox_path).unwrap_or_default();
        let now = crate::provider_activity::unix_millis();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.identity = Some(identity.clone());
        state.outbox = outbox
            .into_iter()
            .filter(|item| now.saturating_sub(item.created_at) <= OUTBOX_MAX_AGE_MILLIS)
            .take(MAX_OUTBOX_ITEMS)
            .collect();
        drop(state);
        let _ = write_json_atomic(&self.identity_path, &identity);
        let _ = self.persist_outbox();
    }

    fn ensure_identity(&self) -> Option<TelemetryIdentity> {
        if !self.enabled.load(Ordering::Acquire) {
            return None;
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.identity.is_none() {
            state.identity = Some(TelemetryIdentity::create());
        }
        let identity = state.identity.clone();
        drop(state);
        if let Some(identity) = identity.as_ref() {
            let _ = write_json_atomic(&self.identity_path, identity);
        }
        identity
    }

    fn enqueue_heartbeat_if_due(&self) {
        let Some(mut identity) = self.ensure_identity() else {
            return;
        };
        let now = crate::provider_activity::unix_millis();
        if now.saturating_sub(identity.last_heartbeat_at) < HEARTBEAT_INTERVAL_MILLIS {
            return;
        }
        let hardware = collect_hardware().ok();
        let hardware_hash = hardware
            .as_ref()
            .and_then(|snapshot| serde_json::to_vec(snapshot).ok())
            .map(|bytes| hex::encode(Sha256::digest(bytes)));
        let changed_hardware = hardware_hash != identity.hardware_hash;
        identity.hardware_hash = hardware_hash;
        identity.last_heartbeat_at = now;
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.identity = Some(identity.clone());
        }
        let _ = write_json_atomic(&self.identity_path, &identity);
        self.enqueue(TelemetryEnvelopeV1 {
            schema_version: 1,
            event_id: format!("heartbeat:{}", now / HEARTBEAT_INTERVAL_MILLIS),
            created_at: now,
            installation_id: identity.installation_id,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            hardware: changed_hardware.then_some(hardware).flatten(),
            performance_daily: Some(self.performance.snapshot()),
            agent_tool_daily: Vec::new(),
        });
    }

    fn enqueue(&self, envelope: TelemetryEnvelopeV1) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = state
            .outbox
            .iter_mut()
            .find(|item| item.event_id == envelope.event_id)
        {
            *existing = envelope;
        } else {
            state.outbox.push(envelope);
        }
        while state.outbox.len() > MAX_OUTBOX_ITEMS
            || serde_json::to_vec(&state.outbox)
                .map(|bytes| bytes.len() > MAX_OUTBOX_BYTES)
                .unwrap_or(true)
        {
            state.outbox.remove(0);
        }
        drop(state);
        let _ = self.persist_outbox();
    }

    fn flush_outbox(&self) {
        let now = crate::provider_activity::unix_millis();
        let (next_retry_at, next) = {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (state.next_retry_at, state.outbox.first().cloned())
        };
        let (Some(endpoint), Some(next)) = (self.endpoint, next) else {
            return;
        };
        if now < next_retry_at {
            return;
        }
        let result = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .and_then(|client| {
                client
                    .post(endpoint)
                    .header("content-type", "application/json")
                    .header("x-itime-schema", "1")
                    .json(&next)
                    .send()
            });
        if result.is_ok_and(|response| response.status().is_success()) {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.outbox.retain(|item| item.event_id != next.event_id);
            state.upload_failures = 0;
            state.next_retry_at = 0;
            drop(state);
            let _ = self.persist_outbox();
        } else {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.upload_failures = state.upload_failures.saturating_add(1);
            let exponent = u32::from(state.upload_failures.min(8));
            let delay_seconds = 30_u64.saturating_mul(2_u64.saturating_pow(exponent));
            state.next_retry_at =
                now.saturating_add(delay_seconds.min(6 * 60 * 60).saturating_mul(1_000));
        }
    }

    fn persist_outbox(&self) -> Result<(), String> {
        let outbox = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .outbox
            .clone();
        write_json_atomic(&self.outbox_path, &outbox)
    }

    fn status(&self) -> TelemetryStatus {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        TelemetryStatus {
            enabled: self.enabled.load(Ordering::Acquire),
            endpoint_configured: self.endpoint.is_some(),
            pending_envelopes: state.outbox.len(),
            last_heartbeat_at: state
                .identity
                .as_ref()
                .map(|identity| identity.last_heartbeat_at)
                .unwrap_or(0),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TelemetryStatus {
    enabled: bool,
    endpoint_configured: bool,
    pending_envelopes: usize,
    last_heartbeat_at: u64,
}

#[tauri::command]
pub(crate) fn mark_ui_ready(telemetry: State<'_, TelemetryService>) {
    telemetry.mark_ui_ready();
}

#[tauri::command]
pub(crate) fn get_telemetry_status(telemetry: State<'_, TelemetryService>) -> TelemetryStatus {
    telemetry.status()
}

fn aggregate_agent_days(intervals: &[ProviderInterval]) -> Vec<AgentToolDaily> {
    let mut grouped = BTreeMap::<(String, String), Vec<(u64, u64, String, bool)>>::new();
    for interval in intervals {
        let mut cursor = interval.start;
        while cursor < interval.end {
            let date = local_date(cursor);
            let Some((day_start, day_end)) = local_day_bounds(&date) else {
                break;
            };
            let clipped_end = interval.end.min(day_end);
            grouped
                .entry((date, interval.tool_id.to_string()))
                .or_default()
                .push((
                    cursor.max(day_start),
                    clipped_end,
                    interval.task_id.clone(),
                    interval.start >= day_start && interval.start < day_end,
                ));
            cursor = clipped_end;
        }
    }
    grouped
        .into_iter()
        .map(|((date, tool_id), rows)| {
            let task_count = rows
                .iter()
                .filter(|(_, _, _, started_today)| *started_today)
                .map(|(_, _, task, _)| task)
                .collect::<HashSet<_>>()
                .len();
            let total_execution_seconds = rows
                .iter()
                .map(|(start, end, _, _)| end.saturating_sub(*start))
                .sum::<u64>()
                / 1_000;
            let mut points = rows
                .iter()
                .flat_map(|(start, end, _, _)| [(*start, 1_i8), (*end, -1_i8)])
                .collect::<Vec<_>>();
            points.sort_by_key(|(at, delta)| (*at, *delta));
            let mut current = 0_i64;
            let mut maximum = 0_i64;
            for (_, delta) in points {
                current += i64::from(delta);
                maximum = maximum.max(current);
            }
            AgentToolDaily {
                date,
                tool_id,
                task_count,
                total_execution_seconds,
                max_concurrency: usize::try_from(maximum).unwrap_or(0),
            }
        })
        .collect()
}

fn local_date(timestamp: u64) -> String {
    i64::try_from(timestamp)
        .ok()
        .and_then(|timestamp| Local.timestamp_millis_opt(timestamp).single())
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

fn local_day_bounds(date: &str) -> Option<(u64, u64)> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let start = Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0)?)
        .earliest()?;
    let next = Local
        .from_local_datetime(&date.succ_opt()?.and_hms_opt(0, 0, 0)?)
        .earliest()?;
    Some((
        u64::try_from(start.timestamp_millis()).ok()?,
        u64::try_from(next.timestamp_millis()).ok()?,
    ))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    atomic_json::write(path, value)
}

#[cfg(windows)]
fn collect_hardware() -> Result<HardwareSnapshot, String> {
    use wmi::WMIConnection;

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ComputerSystem {
        manufacturer: Option<String>,
        model: Option<String>,
        #[serde(rename = "PCSystemType")]
        pc_system_type: Option<u16>,
        total_physical_memory: Option<u64>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Processor {
        name: Option<String>,
        manufacturer: Option<String>,
        architecture: Option<u16>,
        number_of_cores: Option<u32>,
        number_of_logical_processors: Option<u32>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct VideoController {
        name: Option<String>,
        adapter_compatibility: Option<String>,
        #[serde(rename = "AdapterRAM")]
        adapter_ram: Option<u64>,
        driver_version: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct OperatingSystem {
        caption: Option<String>,
        version: Option<String>,
        build_number: Option<String>,
    }

    let connection = WMIConnection::new().map_err(|error| error.to_string())?;
    let system = connection
        .raw_query::<ComputerSystem>(
            "SELECT Manufacturer, Model, PCSystemType, TotalPhysicalMemory FROM Win32_ComputerSystem",
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .next();
    let cpu = connection
        .raw_query::<Processor>(
            "SELECT Name, Manufacturer, Architecture, NumberOfCores, NumberOfLogicalProcessors FROM Win32_Processor",
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .next();
    let gpus = connection
        .raw_query::<VideoController>(
            "SELECT Name, AdapterCompatibility, AdapterRAM, DriverVersion FROM Win32_VideoController",
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|gpu| GpuSnapshot {
            name: gpu.name,
            vendor: gpu.adapter_compatibility,
            memory_bytes: gpu.adapter_ram,
            driver_version: gpu.driver_version,
        })
        .collect();
    let windows = connection
        .raw_query::<OperatingSystem>(
            "SELECT Caption, Version, BuildNumber FROM Win32_OperatingSystem",
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .next();
    Ok(HardwareSnapshot {
        manufacturer: system.as_ref().and_then(|value| value.manufacturer.clone()),
        model: system.as_ref().and_then(|value| value.model.clone()),
        form_factor: match system.as_ref().and_then(|value| value.pc_system_type) {
            Some(2) => "laptop",
            Some(1 | 3 | 4 | 5 | 6 | 7) => "desktop",
            _ => "unknown",
        }
        .to_string(),
        cpu: CpuSnapshot {
            name: cpu.as_ref().and_then(|value| value.name.clone()),
            vendor: cpu.as_ref().and_then(|value| value.manufacturer.clone()),
            architecture: cpu
                .as_ref()
                .and_then(|value| value.architecture)
                .map(|value| value.to_string()),
            physical_cores: cpu.as_ref().and_then(|value| value.number_of_cores),
            logical_threads: cpu
                .as_ref()
                .and_then(|value| value.number_of_logical_processors),
        },
        memory_bytes: system.and_then(|value| value.total_physical_memory),
        gpus,
        windows_version: windows.map(|value| {
            [value.caption, value.version, value.build_number]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
        }),
    })
}

#[cfg(not(windows))]
fn collect_hardware() -> Result<HardwareSnapshot, String> {
    Err("当前仅支持 Windows 硬件采集".into())
}

#[cfg(windows)]
fn peak_working_set_bytes() -> Option<u64> {
    use std::mem::size_of;
    use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;

    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>()).ok()?,
        ..Default::default()
    };
    let success =
        unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    success
        .as_bool()
        .then(|| u64::try_from(counters.PeakWorkingSetSize).ok())
        .flatten()
}

#[cfg(not(windows))]
fn peak_working_set_bytes() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(tool_id: &'static str, task_id: &str, start: u64, end: u64) -> ProviderInterval {
        ProviderInterval {
            version: 1,
            start,
            end,
            provider: tool_id,
            tool_id,
            tool_name: tool_id,
            agent_id: "anonymous".into(),
            task_id: task_id.into(),
            status: "completed",
            basis: "verified-test",
            confidence: 1.0,
        }
    }

    #[test]
    fn daily_aggregation_counts_unique_tasks_duration_and_concurrency() {
        let start = Local
            .with_ymd_and_hms(2026, 7, 20, 10, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let rows = aggregate_agent_days(&[
            interval("codex", "one", start, start + 120_000),
            interval("codex", "two", start + 60_000, start + 180_000),
        ]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].task_count, 2);
        assert_eq!(rows[0].total_execution_seconds, 240);
        assert_eq!(rows[0].max_concurrency, 2);
    }

    #[test]
    fn cross_day_task_is_counted_only_on_its_start_date() {
        let start = Local
            .with_ymd_and_hms(2026, 7, 20, 23, 59, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let rows = aggregate_agent_days(&[interval("codex", "overnight", start, start + 120_000)]);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].task_count, 1);
        assert_eq!(rows[0].total_execution_seconds, 60);
        assert_eq!(rows[1].task_count, 0);
        assert_eq!(rows[1].total_execution_seconds, 60);
    }

    #[test]
    fn telemetry_json_has_no_forbidden_content_or_path_fields() {
        let envelope = TelemetryEnvelopeV1 {
            schema_version: 1,
            event_id: "daily".into(),
            created_at: 1,
            installation_id: "anonymous".into(),
            app_version: "0.2.0".into(),
            hardware: None,
            performance_daily: None,
            agent_tool_daily: vec![AgentToolDaily {
                date: "2026-07-20".into(),
                tool_id: "codex".into(),
                task_count: 1,
                total_execution_seconds: 1,
                max_concurrency: 1,
            }],
        };
        let json = serde_json::to_string(&envelope).unwrap();
        for forbidden_field in [
            "username",
            "serialNumber",
            "macAddress",
            "path",
            "windowTitle",
            "prompt",
            "response",
            "content",
            "code",
        ] {
            assert!(!json.contains(&format!("\"{forbidden_field}\":")));
        }
    }

    #[cfg(windows)]
    #[test]
    fn collects_local_hardware_without_direct_identifiers() {
        let hardware = collect_hardware().unwrap();
        assert!(hardware.cpu.name.is_some());
        assert!(!hardware.gpus.is_empty());
        let json = serde_json::to_string(&hardware).unwrap().to_lowercase();
        for forbidden in ["serialnumber", "macaddress", "username", "deviceid"] {
            assert!(!json.contains(forbidden));
        }
    }
}
