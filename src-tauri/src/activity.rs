mod capture;
mod collector;
mod model;
mod storage;

pub(crate) use collector::ActivityCollector;
use model::{ActivityError, ActivitySnapshot};
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
