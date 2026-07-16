use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyToday {
    pub(super) date: String,
    pub(super) key_strokes: u64,
    pub(super) left_clicks: u64,
    pub(super) right_clicks: u64,
    pub(super) mouse_distance: f64,
    pub(super) scroll_distance: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyHistoryDay {
    pub(super) date: String,
    pub(super) key_strokes: u64,
    pub(super) clicks: u64,
    pub(super) mouse_distance: f64,
    pub(super) scroll_distance: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyKeyStatsFile {
    pub(super) today: LegacyToday,
    #[serde(default)]
    pub(super) history: Vec<LegacyHistoryDay>,
    #[serde(default)]
    pub(super) key_stats: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyStatsToday {
    pub(super) date: String,
    pub(super) key_strokes: u64,
    pub(super) left_clicks: u64,
    pub(super) right_clicks: u64,
    pub(super) mouse_distance: f64,
    pub(super) scroll_distance: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyStatsHistoryDay {
    pub(super) date: String,
    pub(super) key_strokes: u64,
    pub(super) clicks: u64,
    pub(super) mouse_distance: f64,
    pub(super) scroll_distance: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyCount {
    pub(super) key: String,
    pub(super) count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyStatsCapabilities {
    pub(super) history_granularity: &'static str,
    pub(super) minute_density: bool,
    pub(super) split_historical_clicks: bool,
    pub(super) sensitive_surface_exclusion: bool,
    pub(super) delete_by_date: bool,
    pub(super) timezone_semantics: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyStatsSnapshot {
    pub(super) source: &'static str,
    pub(super) updated_at: u128,
    pub(super) today: KeyStatsToday,
    pub(super) history: Vec<KeyStatsHistoryDay>,
    pub(super) single_keys: Vec<KeyCount>,
    pub(super) shortcuts: Vec<KeyCount>,
    pub(super) capabilities: KeyStatsCapabilities,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyStatsReadError {
    pub(super) code: &'static str,
    pub(super) message: String,
}
