use super::model::{ActivityError, ActivitySlice, ActivitySnapshot};
use crate::data_files::{self, ACTIVITY_PREFIX};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

const MAX_QUERY_MILLIS: u64 = 32 * 24 * 60 * 60 * 1_000;

pub(super) fn append_slice_once(slice: &ActivitySlice) -> Result<(), ActivityError> {
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

fn visit_records(
    paths: &[PathBuf],
    mut visitor: impl FnMut(&ActivitySlice) -> Result<(), String>,
) -> Result<(usize, usize, u64), ActivityError> {
    let mut records = 0;
    let mut skipped = 0;
    let mut updated_at = 0;
    for path in paths {
        updated_at = updated_at.max(data_files::modified_millis(path));
        // A shard that cannot be opened (AV lock, deleted between enumeration
        // and open) is counted as skipped instead of failing the whole query.
        let Ok(file) = File::open(path) else {
            skipped += 1;
            continue;
        };
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
            visitor(&slice).map_err(ActivityError::io)?;
            records += 1;
        }
    }
    Ok((records, skipped, updated_at))
}

pub(crate) fn visit_records_from(
    root: &Path,
    visitor: impl FnMut(&ActivitySlice) -> Result<(), String>,
) -> Result<(usize, usize, u64), ActivityError> {
    let paths = data_files::record_files_in(root, ACTIVITY_PREFIX).map_err(ActivityError::io)?;
    visit_records(&paths, visitor)
}

fn visit_records_in_range(
    root: &Path,
    start: u64,
    end: u64,
    visitor: impl FnMut(&ActivitySlice) -> Result<(), String>,
) -> Result<(usize, usize, u64), ActivityError> {
    let paths = data_files::record_files_covering(root, ACTIVITY_PREFIX, start, end)
        .map_err(ActivityError::io)?;
    visit_records(&paths, visitor)
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

fn first_record_start(path: &Path) -> Option<u64> {
    let file = File::open(path).ok()?;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        if let Ok(slice) = serde_json::from_str::<ActivitySlice>(&line) {
            if slice.version == 1 && slice.end > slice.start {
                return Some(slice.start);
            }
        }
    }
    None
}

/// Earliest record start across the whole data directory. Shard names are
/// sorted chronologically, so only the first dated shard plus any undated
/// legacy files need a (single-line) read — the snapshot itself is pruned to
/// the query window and cannot derive this field from its own scan.
fn earliest_record_start(root: &Path) -> Option<u64> {
    let files = data_files::record_files_in(root, ACTIVITY_PREFIX).ok()?;
    let mut earliest = None;
    let mut dated_seen = false;
    for path in &files {
        if data_files::shard_date(path, ACTIVITY_PREFIX).is_some() {
            if dated_seen {
                continue;
            }
            dated_seen = true;
        }
        let Some(start) = first_record_start(path) else {
            continue;
        };
        earliest = Some(earliest.map_or(start, |value: u64| value.min(start)));
    }
    earliest
}

pub(super) fn read_snapshot(start: u64, end: u64) -> Result<ActivitySnapshot, ActivityError> {
    if start >= end || end - start > MAX_QUERY_MILLIS {
        return Err(ActivityError::invalid_range());
    }
    let root = data_files::data_dir().map_err(ActivityError::io)?;
    let recorded_from = earliest_record_start(&root);
    let mut records = Vec::new();
    let (_, skipped, updated_at) = visit_records_in_range(&root, start, end, |slice| {
        records.push(slice.clone());
        Ok(())
    })?;
    records.sort_by_key(|slice| (slice.start, slice.end, slice.generation));
    let mut intervals: Vec<ActivitySlice> = Vec::new();
    for slice in records {
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
    use chrono::TimeZone;
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
        let mut records = Vec::new();
        let (_, skipped, _) = visit_records_from(&root, |slice| {
            records.push(slice.clone());
            Ok(())
        })
        .unwrap();
        records.sort_by_key(|slice| (slice.start, slice.end));
        let _ = fs::remove_dir_all(root);
        assert_eq!(records.len(), 2);
        assert_eq!(skipped, 1);
        assert_eq!((records[0].start, records[1].start), (0, 10));
    }

    #[test]
    fn ranged_read_prunes_shards_outside_the_window() {
        let root = fixture_root("ranged");
        fs::create_dir_all(&root).unwrap();
        let day = chrono::Local
            .with_ymd_and_hms(2026, 7, 27, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let other = chrono::Local
            .with_ymd_and_hms(2026, 7, 20, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        fs::write(
            root.join("activity-2026-07-27-v1.jsonl"),
            format!(
                "{}\n",
                serde_json::to_string(&slice(day, day + 1_000)).unwrap()
            ),
        )
        .unwrap();
        fs::write(
            root.join("activity-2026-07-20-v1.jsonl"),
            format!(
                "{}\n",
                serde_json::to_string(&slice(other, other + 1_000)).unwrap()
            ),
        )
        .unwrap();
        let day_start = chrono::Local
            .with_ymd_and_hms(2026, 7, 27, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let mut records = Vec::new();
        let (_, skipped, _) = visit_records_in_range(
            &root,
            day_start,
            day_start + 24 * 60 * 60 * 1_000,
            |slice| {
                records.push(slice.clone());
                Ok(())
            },
        )
        .unwrap();
        let _ = fs::remove_dir_all(root);
        assert_eq!(skipped, 0);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].start, day);
    }

    #[test]
    fn visitor_streams_valid_records_and_preserves_skip_count() {
        let root = fixture_root("visitor");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("activity-v1.jsonl"),
            format!(
                "{}\nbad-line\n{}\n",
                serde_json::to_string(&slice(10, 20)).unwrap(),
                serde_json::to_string(&slice(20, 30)).unwrap()
            ),
        )
        .unwrap();
        let mut starts = Vec::new();
        let (records, skipped, _) = visit_records_from(&root, |slice| {
            starts.push(slice.start);
            Ok(())
        })
        .unwrap();
        let _ = fs::remove_dir_all(root);
        assert_eq!(records, 2);
        assert_eq!(skipped, 1);
        assert_eq!(starts, vec![10, 20]);
    }
}
