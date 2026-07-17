use super::model::{ActivityError, ActivitySlice, ActivitySnapshot};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    thread,
    time::{Duration, SystemTime},
};

const MAX_QUERY_MILLIS: u64 = 32 * 24 * 60 * 60 * 1_000;

fn data_file_path() -> Result<PathBuf, ActivityError> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| ActivityError {
        code: "not_found",
        message: "Windows LOCALAPPDATA 路径不可用".into(),
    })?;
    Ok(PathBuf::from(local)
        .join("iTime")
        .join("Data")
        .join("activity-v1.jsonl"))
}

fn append_slice_once(slice: &ActivitySlice) -> Result<(), ActivityError> {
    let path = data_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(ActivityError::io)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(ActivityError::io)?;
    serde_json::to_writer(&mut file, slice).map_err(ActivityError::io)?;
    file.write_all(b"\n").map_err(ActivityError::io)?;
    file.flush().map_err(ActivityError::io)?;
    file.sync_data().map_err(ActivityError::io)
}

pub(super) fn append_slice(slice: &ActivitySlice) -> Result<(), ActivityError> {
    let mut last_error = None;
    for attempt in 0..3 {
        match append_slice_once(slice) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        if attempt < 2 {
            thread::sleep(Duration::from_millis(50 * (attempt + 1)));
        }
    }
    Err(last_error.unwrap_or_else(|| ActivityError::io("活动记录写入失败")))
}

fn modified_millis(path: &PathBuf) -> u128 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|value| value.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_millis())
}

fn can_merge(previous: &ActivitySlice, next: &ActivitySlice) -> bool {
    previous.end == next.start && previous.observation == next.observation
}

fn clip(mut slice: ActivitySlice, start: u64, end: u64) -> Option<ActivitySlice> {
    if slice.end <= start || slice.start >= end || slice.end <= slice.start {
        return None;
    }
    slice.start = slice.start.max(start);
    slice.end = slice.end.min(end);
    Some(slice)
}

pub(super) fn read_snapshot(start: u64, end: u64) -> Result<ActivitySnapshot, ActivityError> {
    if start >= end || end - start > MAX_QUERY_MILLIS {
        return Err(ActivityError::invalid_range());
    }
    let path = data_file_path()?;
    if !path.is_file() {
        return Ok(ActivitySnapshot::new(0, None, 0, Vec::new()));
    }
    let file = OpenOptions::new()
        .read(true)
        .open(&path)
        .map_err(ActivityError::io)?;
    let mut intervals: Vec<ActivitySlice> = Vec::new();
    let mut skipped = 0;
    let mut recorded_from = None;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            skipped += 1;
            continue;
        };
        let Ok(slice) = serde_json::from_str::<ActivitySlice>(&line) else {
            skipped += 1;
            continue;
        };
        if slice.version != 1 {
            skipped += 1;
            continue;
        }
        recorded_from =
            Some(recorded_from.map_or(slice.start, |value: u64| value.min(slice.start)));
        let Some(slice) = clip(slice, start, end) else {
            continue;
        };
        if let Some(previous) = intervals.last_mut() {
            if can_merge(previous, &slice) {
                previous.end = slice.end;
                continue;
            }
        }
        intervals.push(slice);
    }
    Ok(ActivitySnapshot::new(
        modified_millis(&path),
        recorded_from,
        skipped,
        intervals,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::model::{ActivityObservation, DeviceState};

    fn slice(start: u64, end: u64) -> ActivitySlice {
        ActivitySlice {
            version: 1,
            start,
            end,
            observation: ActivityObservation {
                device_state: DeviceState::Active,
                app_id: Some("code".into()),
                app_name: Some("Code".into()),
                ai_tool: false,
            },
        }
    }

    #[test]
    fn clips_and_merges_adjacent_equal_observations() {
        let mut first = clip(slice(0, 10), 5, 20).expect("overlap");
        let second = clip(slice(10, 25), 5, 20).expect("overlap");
        assert!(can_merge(&first, &second));
        first.end = second.end;
        assert_eq!((first.start, first.end), (5, 20));
    }
}
