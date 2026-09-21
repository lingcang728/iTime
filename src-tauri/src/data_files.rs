use chrono::{Local, NaiveDate, TimeZone};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const ACTIVITY_PREFIX: &str = "activity";
pub(crate) const KEYBOARD_PREFIX: &str = "keyboard";
const SCHEMA_SUFFIX: &str = "-v1.jsonl";
const MAX_SHARD_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn data_dir() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows LOCALAPPDATA 路径不可用".to_string())?;
    Ok(PathBuf::from(local).join("iTime").join("Data"))
}

fn date_for(timestamp: u64) -> Result<NaiveDate, String> {
    let millis = i64::try_from(timestamp).map_err(|_| "记录时间超出支持范围".to_string())?;
    Local
        .timestamp_millis_opt(millis)
        .single()
        .map(|value| value.date_naive())
        .ok_or_else(|| "记录时间无法转换为本地日期".to_string())
}

fn shard_name(prefix: &str, date: NaiveDate, part: u32) -> String {
    if part == 1 {
        format!("{prefix}-{date}{SCHEMA_SUFFIX}")
    } else {
        format!("{prefix}-{date}-part{part}{SCHEMA_SUFFIX}")
    }
}

pub(crate) fn writable_shard(
    root: &Path,
    prefix: &str,
    timestamp: u64,
    record_bytes: usize,
) -> Result<PathBuf, String> {
    let date = date_for(timestamp)?;
    for part in 1..=9_999 {
        let candidate = root.join(shard_name(prefix, date, part));
        let current_size = fs::metadata(&candidate).map_or(0, |metadata| metadata.len());
        if current_size == 0 || current_size.saturating_add(record_bytes as u64) <= MAX_SHARD_BYTES
        {
            return Ok(candidate);
        }
    }
    Err("当天数据分片数量超出支持范围".into())
}

pub(crate) fn append_json_line(
    root: &Path,
    prefix: &str,
    timestamp: u64,
    json: &[u8],
) -> Result<PathBuf, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let mut line = Vec::with_capacity(json.len() + 1);
    line.extend_from_slice(json);
    line.push(b'\n');
    let path = writable_shard(root, prefix, timestamp, line.len())?;
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    let existing_len = file.metadata().map_err(|error| error.to_string())?.len();
    if existing_len > 0 {
        file.seek(SeekFrom::End(-1))
            .map_err(|error| error.to_string())?;
        let mut last = [0u8; 1];
        file.read_exact(&mut last)
            .map_err(|error| error.to_string())?;
        if last[0] != b'\n' {
            // Isolate an interrupted final record so the next complete record remains readable.
            file.write_all(b"\n").map_err(|error| error.to_string())?;
        }
    }
    // Serialize before opening the file, then append the complete bounded record in one call.
    // Readers recover at newline boundaries if the process is interrupted during a write.
    file.write_all(&line).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    file.sync_data().map_err(|error| error.to_string())?;
    Ok(path)
}

/// Strict record naming: only shapes the writer itself produces — legacy
/// `{prefix}-v1.jsonl` and dated `{prefix}-YYYY-MM-DD[-part<N>]-v1.jsonl`.
/// A looser prefix match lets a same-user process drop arbitrary
/// `activity-*.jsonl` files that readers would parse and render as forged
/// history.
fn is_record_name(name: &str, prefix: &str) -> bool {
    let Some(rest) = name.strip_prefix(&format!("{prefix}-")) else {
        return false;
    };
    if rest == "v1.jsonl" {
        return true;
    }
    let Some(core) = rest.strip_suffix(SCHEMA_SUFFIX) else {
        return false;
    };
    // `get` instead of slicing: a non-ASCII filename must not panic here.
    let Some(date) = core.get(..10) else {
        return false;
    };
    if NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
        return false;
    }
    match &core[10..] {
        "" => true,
        tail => tail.strip_prefix("-part").is_some_and(|digits| {
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        }),
    }
}

pub(crate) fn record_files_in(root: &Path, prefix: &str) -> Result<Vec<PathBuf>, String> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        // DirEntry::file_type never follows links: a planted `activity-*.jsonl`
        // symlink is neither listed nor ever opened by readers.
        let is_file = entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false);
        if is_file
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| is_record_name(name, prefix))
        {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

/// Files carrying a record prefix that do NOT match a writer-produced name —
/// planted or stale residue. Readers never list them; retention cleanup and
/// "删除全部" sweep them so they cannot linger as forgeable material.
fn foreign_record_files_in(root: &Path, prefix: &str) -> Result<Vec<PathBuf>, String> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let is_file = entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false);
        if !is_file {
            continue;
        }
        // Only the `-v1.jsonl` suffix was ever readable under the old loose
        // naming rule, so only those foreign names are worth sweeping — a
        // future `{prefix}-*-v2.jsonl` written by a newer version must survive
        // a downgrade's cleanup pass.
        let foreign = entry.file_name().to_str().is_some_and(|name| {
            name.starts_with(&format!("{prefix}-"))
                && name.ends_with(SCHEMA_SUFFIX)
                && !is_record_name(name, prefix)
        });
        if foreign {
            paths.push(entry.path());
        }
    }
    Ok(paths)
}

/// Prefixed non-record files across both record families.
pub(crate) fn foreign_record_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = foreign_record_files_in(root, ACTIVITY_PREFIX)?;
    files.extend(foreign_record_files_in(root, KEYBOARD_PREFIX)?);
    Ok(files)
}

pub(crate) fn all_record_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = record_files_in(root, ACTIVITY_PREFIX)?;
    files.extend(record_files_in(root, KEYBOARD_PREFIX)?);
    files.sort();
    Ok(files)
}

/// Shards are named by the local date of the records' `start` timestamp, so a
/// query window only needs the shards whose date falls inside `[start, end)`.
/// Undated legacy files (`{prefix}-v1.jsonl`) and unparsable names are always
/// included so nothing readable is ever pruned away.
pub(crate) fn record_files_covering(
    root: &Path,
    prefix: &str,
    start: u64,
    end: u64,
) -> Result<Vec<PathBuf>, String> {
    let files = record_files_in(root, prefix)?;
    let (Some(first), Some(last)) = (
        date_for(start).ok(),
        date_for(end.saturating_sub(1).max(start)).ok(),
    ) else {
        return Ok(files);
    };
    Ok(files
        .into_iter()
        .filter(|path| shard_date(path, prefix).is_none_or(|date| date >= first && date <= last))
        .collect())
}

pub(crate) fn shard_date(path: &Path, prefix: &str) -> Option<NaiveDate> {
    let name = path.file_name()?.to_str()?;
    let remainder = name.strip_prefix(&format!("{prefix}-"))?;
    if remainder == "v1.jsonl" || remainder.len() < 10 {
        return None;
    }
    // `get` instead of slicing: a non-ASCII filename must not panic here.
    NaiveDate::parse_from_str(remainder.get(..10)?, "%Y-%m-%d").ok()
}

pub(crate) fn cleanup_expired_in(
    root: &Path,
    retention_days: Option<u16>,
    today: NaiveDate,
) -> Result<usize, String> {
    let cutoff = retention_days
        .map(|days| today - chrono::Duration::days(i64::from(days.saturating_sub(1))));
    // A shard dated beyond tomorrow was produced by a badly skewed clock and can
    // never age out of a retention window or receive new writes — sweep it.
    let future_limit = today + chrono::Duration::days(1);
    let mut removed = 0;
    for prefix in [ACTIVITY_PREFIX, KEYBOARD_PREFIX] {
        let legacy_name = format!("{prefix}-v1.jsonl");
        for path in record_files_in(root, prefix)? {
            let remove = match shard_date(&path, prefix) {
                Some(date) => {
                    // The current local-date shard is always active and must
                    // never be removed.
                    date > future_limit
                        || cutoff.is_some_and(|cutoff| date < cutoff && date != today)
                }
                None => {
                    // Only the exact pre-sharding legacy file is preserved
                    // forever; any other undated file carrying a record prefix
                    // is foreign residue that would otherwise linger forever.
                    path.file_name().and_then(|name| name.to_str()) != Some(legacy_name.as_str())
                }
            };
            if !remove {
                continue;
            }
            // 文件名即可定位问题；不回显完整路径，避免把用户名泄露进 UI。
            let label = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("未知分片")
                .to_string();
            fs::remove_file(&path).map_err(|error| format!("无法清理 {label}：{error}"))?;
            removed += 1;
        }
        // Foreign `{prefix}-*-v1.jsonl` residue (planted or stale junk that
        // fails strict record naming) is swept in every retention pass so it
        // cannot linger as forgeable history material.
        for path in foreign_record_files_in(root, prefix)? {
            let label = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("未知分片")
                .to_string();
            fs::remove_file(&path).map_err(|error| format!("无法清理 {label}：{error}"))?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub(crate) fn cleanup_expired(retention_days: Option<u16>) -> Result<usize, String> {
    cleanup_expired_in(&data_dir()?, retention_days, Local::now().date_naive())
}

/// Copies every readable `version:1` line out of the staging file into dated
/// shards. `existing` tracks exact-line multiplicities already present in the
/// shards so an interrupted run resumes without duplicating, and is updated as
/// lines are copied so later staging files dedupe against earlier ones.
fn migrate_pending_lines(
    root: &Path,
    prefix: &str,
    pending: &Path,
    existing: &mut HashMap<String, usize>,
    unreadable: &mut Vec<String>,
) -> Result<usize, String> {
    let mut seen = HashMap::<String, usize>::new();
    let mut migrated = 0;
    for line in BufReader::new(fs::File::open(pending).map_err(|error| error.to_string())?).lines()
    {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                unreadable.push(format!("读取失败：{error}"));
                continue;
            }
        };
        let timestamp = serde_json::from_str::<serde_json::Value>(&line)
            .ok()
            .filter(|value| value.get("version").and_then(serde_json::Value::as_u64) == Some(1))
            .and_then(|value| value.get("start").and_then(serde_json::Value::as_u64));
        let Some(timestamp) = timestamp else {
            unreadable.push(line);
            continue;
        };
        let occurrence = seen.entry(line.clone()).or_default();
        *occurrence += 1;
        if *occurrence <= existing.get(&line).copied().unwrap_or(0) {
            continue;
        }
        append_json_line(root, prefix, timestamp, line.as_bytes())?;
        *existing.entry(line).or_default() += 1;
        migrated += 1;
    }
    Ok(migrated)
}

pub(crate) fn migrate_legacy_file_in(root: &Path, prefix: &str) -> Result<usize, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let legacy = root.join(format!("{prefix}-v1.jsonl"));
    let pending = root.join(format!(".{prefix}-v1.migrating"));

    // Exact-line multiplicities make an interrupted migration resumable without
    // duplicating records already copied before the interruption. A shard that
    // cannot be opened right now must not abort the whole migration; worst case
    // its duplicates are copied again and skipped by readers as exact repeats.
    let mut existing = HashMap::<String, usize>::new();
    for path in record_files_in(root, prefix)? {
        // The legacy file itself is a migration source, not a destination:
        // counting its lines would make every pending line look already-copied.
        if path == legacy {
            continue;
        }
        let Ok(file) = fs::File::open(&path) else {
            continue;
        };
        for line in BufReader::new(file).lines() {
            let Ok(line) = line else {
                break;
            };
            *existing.entry(line).or_default() += 1;
        }
    }

    // An interrupted migration can leave `pending` behind while a downgrade or
    // a restored backup recreates `legacy`. Fold both into the staging path one
    // at a time — pending first since it was already mid-flight — instead of
    // deadlocking on their coexistence.
    let mut unreadable = Vec::new();
    let mut migrated = 0;
    for source in [pending.clone(), legacy] {
        if !source.is_file() {
            continue;
        }
        if source != pending {
            fs::rename(&source, &pending)
                .map_err(|error| format!("无法关闭旧版 {prefix} 数据文件：{error}"))?;
        }
        migrated += migrate_pending_lines(root, prefix, &pending, &mut existing, &mut unreadable)?;
        fs::remove_file(&pending).map_err(|error| error.to_string())?;
    }

    if !unreadable.is_empty() {
        let recovery = root.join("Recovery");
        fs::create_dir_all(&recovery).map_err(|error| error.to_string())?;
        let path = recovery.join(format!(
            "{prefix}-legacy-unreadable-{}.jsonl",
            unix_millis()
        ));
        let mut file = fs::File::create(&path).map_err(|error| error.to_string())?;
        for line in unreadable {
            file.write_all(line.as_bytes())
                .and_then(|()| file.write_all(b"\n"))
                .map_err(|error| error.to_string())?;
        }
        file.sync_all().map_err(|error| error.to_string())?;
    }
    Ok(migrated)
}

pub(crate) fn migrate_legacy_files() -> Result<usize, String> {
    let root = data_dir()?;
    let activity = migrate_legacy_file_in(&root, ACTIVITY_PREFIX)?;
    let keyboard = migrate_legacy_file_in(&root, KEYBOARD_PREFIX)?;
    Ok(activity + keyboard)
}

pub(crate) fn modified_millis(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(system_millis)
        .unwrap_or(0)
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

    fn fixture_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "itime-data-files-{name}-{}-{}",
            std::process::id(),
            unix_millis()
        ))
    }

    #[test]
    fn shard_name_contains_local_record_date() {
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let path = writable_shard(Path::new("data"), ACTIVITY_PREFIX, timestamp, 100).unwrap();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("activity-2026-07-27-v1.jsonl")
        );
    }

    #[test]
    fn retention_never_deletes_current_or_undated_legacy_files() {
        let root = fixture_root("retention");
        fs::create_dir_all(&root).unwrap();
        for name in [
            "activity-v1.jsonl",
            "activity-2026-04-01-v1.jsonl",
            "activity-2026-07-27-v1.jsonl",
            "keyboard-2026-04-01-v1.jsonl",
            "keyboard-2026-07-27-part2-v1.jsonl",
        ] {
            fs::write(root.join(name), b"{}\n").unwrap();
        }
        let removed = cleanup_expired_in(
            &root,
            Some(90),
            NaiveDate::from_ymd_opt(2026, 7, 27).unwrap(),
        )
        .unwrap();
        assert_eq!(removed, 2);
        assert!(root.join("activity-v1.jsonl").is_file());
        assert!(root.join("activity-2026-07-27-v1.jsonl").is_file());
        assert!(root.join("keyboard-2026-07-27-part2-v1.jsonl").is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn retention_keeps_exactly_the_requested_calendar_window() {
        let root = fixture_root("retention-boundary");
        fs::create_dir_all(&root).unwrap();
        let expired = root.join("activity-2026-04-28-v1.jsonl");
        let oldest_kept = root.join("activity-2026-04-29-v1.jsonl");
        fs::write(&expired, b"{}\n").unwrap();
        fs::write(&oldest_kept, b"{}\n").unwrap();

        let removed = cleanup_expired_in(
            &root,
            Some(90),
            NaiveDate::from_ymd_opt(2026, 7, 27).unwrap(),
        )
        .unwrap();

        assert_eq!(removed, 1);
        assert!(!expired.exists());
        assert!(oldest_kept.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn permanent_retention_removes_nothing() {
        let root = fixture_root("permanent");
        fs::create_dir_all(&root).unwrap();
        let old = root.join("activity-2020-01-01-v1.jsonl");
        fs::write(&old, b"{}\n").unwrap();
        assert_eq!(
            cleanup_expired_in(&root, None, NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()).unwrap(),
            0
        );
        assert!(old.is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn append_isolates_an_interrupted_tail_before_the_next_record() {
        let root = fixture_root("tail-recovery");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let path = writable_shard(&root, ACTIVITY_PREFIX, timestamp, 20).unwrap();
        fs::write(&path, b"{\"interrupted\":").unwrap();
        append_json_line(&root, ACTIVITY_PREFIX, timestamp, br#"{"version":1}"#).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text, "{\"interrupted\":\n{\"version\":1}\n");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn capacity_rotation_uses_a_closed_part_and_keeps_today_writable() {
        let root = fixture_root("capacity");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let first = root.join("activity-2026-07-27-v1.jsonl");
        fs::File::create(&first)
            .unwrap()
            .set_len(MAX_SHARD_BYTES)
            .unwrap();
        let next = writable_shard(&root, ACTIVITY_PREFIX, timestamp, 20).unwrap();
        assert_eq!(
            next.file_name().and_then(|name| name.to_str()),
            Some("activity-2026-07-27-part2-v1.jsonl")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_migration_preserves_duplicates_and_quarantines_bad_lines() {
        let root = fixture_root("legacy");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let line = format!(r#"{{"version":1,"start":{timestamp},"keyStrokes":2}}"#);
        fs::write(
            root.join("keyboard-v1.jsonl"),
            format!("{line}\n{line}\nnot-json\n"),
        )
        .unwrap();
        assert_eq!(migrate_legacy_file_in(&root, KEYBOARD_PREFIX).unwrap(), 2);
        assert!(!root.join("keyboard-v1.jsonl").exists());
        let shard = root.join("keyboard-2026-07-27-v1.jsonl");
        assert_eq!(fs::read_to_string(shard).unwrap().lines().count(), 2);
        assert_eq!(fs::read_dir(root.join("Recovery")).unwrap().count(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn covering_files_prune_shards_outside_the_query_window() {
        let root = fixture_root("covering");
        fs::create_dir_all(&root).unwrap();
        for name in [
            "activity-2026-07-20-v1.jsonl",
            "activity-2026-07-27-v1.jsonl",
            "activity-2026-08-01-part2-v1.jsonl",
            "activity-v1.jsonl",
        ] {
            fs::write(root.join(name), b"{}\n").unwrap();
        }
        let start = Local
            .with_ymd_and_hms(2026, 7, 27, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let end = Local
            .with_ymd_and_hms(2026, 7, 28, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let files = record_files_covering(&root, ACTIVITY_PREFIX, start, end).unwrap();
        let names: Vec<_> = files
            .iter()
            .map(|path| path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        let _ = fs::remove_dir_all(root);
        assert_eq!(
            names,
            vec![
                "activity-2026-07-27-v1.jsonl".to_string(),
                "activity-v1.jsonl".to_string()
            ]
        );
    }

    #[test]
    fn retention_sweeps_future_and_undated_residue_but_keeps_legacy() {
        let root = fixture_root("retention-anomalies");
        fs::create_dir_all(&root).unwrap();
        for name in [
            "activity-v1.jsonl",
            "activity-x-v1.jsonl",
            "activity-2030-01-01-v1.jsonl",
            "activity-2026-07-27-v1.jsonl",
            "activity-2026-07-28-v1.jsonl",
        ] {
            fs::write(root.join(name), b"{}\n").unwrap();
        }
        let removed =
            cleanup_expired_in(&root, None, NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()).unwrap();
        assert_eq!(removed, 2);
        assert!(root.join("activity-v1.jsonl").is_file());
        assert!(root.join("activity-2026-07-27-v1.jsonl").is_file());
        assert!(root.join("activity-2026-07-28-v1.jsonl").is_file());
        assert!(!root.join("activity-x-v1.jsonl").exists());
        assert!(!root.join("activity-2030-01-01-v1.jsonl").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn migration_folds_reappeared_legacy_file_into_pending() {
        let root = fixture_root("legacy-coexist");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let pending_line = format!(r#"{{"version":1,"start":{timestamp},"keyStrokes":2}}"#);
        let legacy_line = format!(
            r#"{{"version":1,"start":{},"keyStrokes":5}}"#,
            timestamp + 60_000
        );
        fs::write(
            root.join(".keyboard-v1.migrating"),
            format!("{pending_line}\n"),
        )
        .unwrap();
        fs::write(
            root.join("keyboard-v1.jsonl"),
            format!("{pending_line}\n{legacy_line}\n"),
        )
        .unwrap();
        assert_eq!(migrate_legacy_file_in(&root, KEYBOARD_PREFIX).unwrap(), 2);
        let shard = root.join("keyboard-2026-07-27-v1.jsonl");
        assert_eq!(fs::read_to_string(shard).unwrap().lines().count(), 2);
        assert!(!root.join(".keyboard-v1.migrating").exists());
        assert!(!root.join("keyboard-v1.jsonl").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn record_names_match_only_writer_shapes() {
        // Shapes the writer actually produces.
        assert!(is_record_name("activity-v1.jsonl", ACTIVITY_PREFIX));
        assert!(is_record_name(
            "activity-2026-07-27-v1.jsonl",
            ACTIVITY_PREFIX
        ));
        assert!(is_record_name(
            "keyboard-2026-07-27-part2-v1.jsonl",
            KEYBOARD_PREFIX
        ));
        // Planted lookalikes are not records — they would otherwise be read
        // and rendered as forged history.
        assert!(!is_record_name("activity-x-v1.jsonl", ACTIVITY_PREFIX));
        assert!(!is_record_name(
            "activity-2026-07-27.jsonl",
            ACTIVITY_PREFIX
        ));
        assert!(!is_record_name(
            "activity-2026-07-27-part-v1.jsonl",
            ACTIVITY_PREFIX
        ));
        assert!(!is_record_name(
            "activity-2026-07-27-evil-v1.jsonl",
            ACTIVITY_PREFIX
        ));
        assert!(!is_record_name(
            "other-2026-07-27-v1.jsonl",
            ACTIVITY_PREFIX
        ));
        // A non-ASCII lookalike must be rejected without panicking on slicing.
        assert!(!is_record_name("activity-中文-v1.jsonl", ACTIVITY_PREFIX));
    }

    #[test]
    fn foreign_residue_is_never_listed_but_is_swept_by_cleanup() {
        let root = fixture_root("foreign");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("activity-forged-v1.jsonl"), b"{}\n").unwrap();
        fs::write(root.join("activity-2026-07-27-v1.jsonl"), b"{}\n").unwrap();
        // A future-schema file must survive a downgrade's sweep.
        fs::write(root.join("activity-v2.jsonl"), b"{}\n").unwrap();

        assert_eq!(
            record_files_in(&root, ACTIVITY_PREFIX).unwrap().len(),
            1,
            "foreign files must never reach record readers"
        );
        let foreign = foreign_record_files(&root).unwrap();
        assert_eq!(foreign.len(), 1);
        assert_eq!(
            foreign[0].file_name().and_then(|n| n.to_str()),
            Some("activity-forged-v1.jsonl")
        );

        let removed =
            cleanup_expired_in(&root, None, NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()).unwrap();
        assert_eq!(removed, 1);
        assert!(root.join("activity-2026-07-27-v1.jsonl").is_file());
        assert!(root.join("activity-v2.jsonl").is_file());
        assert!(!root.join("activity-forged-v1.jsonl").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn interrupted_legacy_migration_skips_already_copied_occurrences() {
        let root = fixture_root("legacy-resume");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let line = format!(r#"{{"version":1,"start":{timestamp},"keyStrokes":2}}"#);
        fs::write(
            root.join(".keyboard-v1.migrating"),
            format!("{line}\n{line}\n"),
        )
        .unwrap();
        fs::write(
            root.join("keyboard-2026-07-27-v1.jsonl"),
            format!("{line}\n"),
        )
        .unwrap();
        assert_eq!(migrate_legacy_file_in(&root, KEYBOARD_PREFIX).unwrap(), 1);
        assert_eq!(
            fs::read_to_string(root.join("keyboard-2026-07-27-v1.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        let _ = fs::remove_dir_all(root);
    }
}
