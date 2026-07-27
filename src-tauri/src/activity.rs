mod capture;
mod collector;
pub(crate) mod model;
mod storage;

pub(crate) use collector::ActivityCollector;
use model::{ActivityError, ActivitySnapshot};
pub(crate) use model::{ActivitySlice, DeviceState};
use tauri::State;

#[tauri::command]
pub(crate) fn get_activity_snapshot(
    collector: State<'_, ActivityCollector>,
    start: u64,
    end: u64,
) -> Result<ActivitySnapshot, ActivityError> {
    let mut snapshot = storage::read_snapshot(start, end)?;
    snapshot.set_health(collector.health());
    Ok(snapshot)
}

pub(crate) fn read_all_records_from(
    root: &std::path::Path,
) -> Result<(Vec<ActivitySlice>, usize, u64), ActivityError> {
    storage::read_all_records_from(root)
}
