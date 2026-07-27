use super::model::{ActivityError, ActivitySlice, ActivitySnapshot};
use crate::data_files::{self, ACTIVITY_PREFIX};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    thread,
    time::Duration,
};

const MAX_QUERY_MILLIS: u64 = 32 * 24 * 60 * 60 * 1_000;

fn append_slice_once(slice: &ActivitySlice) -> Result<(), ActivityError> {
    let root = data_files::data_dir().map_err(ActivityError::io)?;
    let json = serde_json::to_vec(slice).map_err(ActivityError::io)?;
    data_files::append_json_line(&root, ACTIVITY_PREFIX, slice.start, &json)
        .map(|_| ())
        .map_err(ActivityError::io)
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

pub(crate) fn read_all_records_from(
    root: &Path,
) -> Result<(Vec<ActivitySlice>, usize, u64), ActivityError> {
    let paths = data_files::record_files_in(root, ACTIVITY_PREFIX).map_err(ActivityError::io)?;
    let mut records = Vec::new();
    let mut skipped = 0;
    let mut updated_at = 0;
    for path in paths {
        updated_at = updated_at.max(data_files::modified_millis(&path));
        let file = File::open(&path).map_err(ActivityError::io)?;
        for line in BufReader::new(file).lines() {
            let Ok(line) = line else {
                skipped += 1;
                continue;
            };
            let Ok(slice) = serde_json::from_str::<ActivitySlice>(&line) else {
                skipped += 1;
                continue;
            };
            if slice.version != 1 || slice.end <= slice.start {
                skipped += 1;
                continue;
            }
            records.push(slice);
        }
    }
    records.sort_by_key(|slice| (slice.start, slice.end, slice.generation));
    Ok((records, skipped, updated_at))
}

pub(crate) fn read_all_records() -> Result<(Vec<ActivitySlice>, usize, u64), ActivityError> {
    let root = data_files::data_dir().map_err(ActivityError::io)?;
    read_all_records_from(&root)
}

fn can_merge(previous: &ActivitySlice, next: &ActivitySlice) -> bool {
    previous.end == next.start
        && previous.generation == next.generation
        && previous.observation == next.observation
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
    let (records, skipped, updated_at) = read_all_records()?;
    let mut intervals: Vec<ActivitySlice> = Vec::new();
    let mut recorded_from = None;
    for slice in records {
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
        u128::from(updated_at),
        recorded_from,
        skipped,
        intervals,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::model::{ActivityObservation, DeviceState};
    use std::fs;
    use std::path::PathBuf;

    fn fixture_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "itime-activity-storage-{name}-{}-{}",
            std::process::id(),
            data_files::unix_millis()
        ))
    }

    fn slice(start: u64, end: u64) -> ActivitySlice {
        ActivitySlice {
            version: 1,
            start,
            end,
            generation: 1,
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

    #[test]
    fn never_merges_equal_observations_across_recording_generations() {
        let first = slice(0, 10);
        let mut second = slice(10, 20);
        second.generation = first.generation + 1;
        assert!(!can_merge(&first, &second));
    }

    #[test]
    fn reads_legacy_and_rotated_files_and_recovers_after_bad_lines() {
        let root = fixture_root("rotated");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("activity-v1.jsonl"),
            format!(
                "{}\nnot-json\n",
                serde_json::to_string(&slice(0, 10)).unwrap()
            ),
        )
        .unwrap();
        fs::write(
            root.join("activity-2026-07-27-v1.jsonl"),
            format!("{}\n", serde_json::to_string(&slice(10, 20)).unwrap()),
        )
        .unwrap();
        let (records, skipped, _) = read_all_records_from(&root).unwrap();
        let _ = fs::remove_dir_all(root);
        assert_eq!(records.len(), 2);
        assert_eq!(skipped, 1);
        assert_eq!((records[0].start, records[1].start), (0, 10));
    }
}
