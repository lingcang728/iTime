use serde::{Deserialize, Serialize};

pub(super) const SAMPLE_INTERVAL_SECONDS: u64 = 10;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum DeviceState {
    Active,
    Idle,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ActivityObservation {
    pub(super) device_state: DeviceState,
    pub(super) app_id: Option<String>,
    pub(super) app_name: Option<String>,
    pub(super) ai_tool: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivitySlice {
    pub(super) version: u8,
    pub(super) start: u64,
    pub(super) end: u64,
    #[serde(flatten)]
    pub(super) observation: ActivityObservation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivityCapabilities {
    content_captured: bool,
    window_titles_captured: bool,
    executable_paths_captured: bool,
    historical_backfill: bool,
    session_states_captured: bool,
    sampling_interval_seconds: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CollectorHealth {
    pub(super) collector_running: bool,
    pub(super) last_write_at: Option<u64>,
    pub(super) last_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivitySnapshot {
    pub(super) source: &'static str,
    pub(super) updated_at: u128,
    pub(super) recorded_from: Option<u64>,
    pub(super) skipped_records: usize,
    pub(super) intervals: Vec<ActivitySlice>,
    pub(super) capabilities: ActivityCapabilities,
    pub(super) health: CollectorHealth,
}

impl ActivitySnapshot {
    pub(super) fn new(
        updated_at: u128,
        recorded_from: Option<u64>,
        skipped_records: usize,
        intervals: Vec<ActivitySlice>,
    ) -> Self {
        Self {
            source: "iTime 本机活动采样 · 从启用后记录",
            updated_at,
            recorded_from,
            skipped_records,
            intervals,
            capabilities: ActivityCapabilities {
                content_captured: false,
                window_titles_captured: false,
                executable_paths_captured: false,
                historical_backfill: false,
                session_states_captured: false,
                sampling_interval_seconds: SAMPLE_INTERVAL_SECONDS,
            },
            health: CollectorHealth {
                collector_running: false,
                last_write_at: None,
                last_error: None,
            },
        }
    }

    pub(super) fn set_health(&mut self, health: CollectorHealth) {
        self.health = health;
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivityError {
    pub(super) code: &'static str,
    pub(super) message: String,
}

impl ActivityError {
    pub(super) fn io(error: impl ToString) -> Self {
        Self {
            code: "io_failed",
            message: error.to_string(),
        }
    }

    pub(super) fn invalid_range() -> Self {
        Self {
            code: "invalid_range",
            message: "活动查询区间无效".into(),
        }
    }
}
