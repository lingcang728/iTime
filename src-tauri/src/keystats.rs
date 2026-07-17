mod model;

use model::*;
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};

const READ_ATTEMPTS: usize = 3;
const RETRY_DELAY: Duration = Duration::from_millis(30);

fn is_iso_date(value: &str) -> bool {
    if !(value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit()))
    {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn invalid_data(message: impl Into<String>) -> KeyStatsReadError {
    KeyStatsReadError {
        code: "invalid_data",
        message: message.into(),
    }
}

fn parse_snapshot(contents: &str, updated_at: u128) -> Result<KeyStatsSnapshot, KeyStatsReadError> {
    let legacy: LegacyKeyStatsFile =
        serde_json::from_str(contents).map_err(|error| invalid_data(error.to_string()))?;
    if !is_iso_date(&legacy.today.date)
        || legacy.history.iter().any(|item| !is_iso_date(&item.date))
    {
        return Err(invalid_data("KeyStats 日期字段格式无效"));
    }

    let mut single_keys = Vec::new();
    let mut shortcuts = Vec::new();
    for (key, count) in legacy.key_stats.into_iter().filter(|(_, count)| *count > 0) {
        let item = KeyCount { key, count };
        if item.key.contains(" + ") {
            shortcuts.push(item);
        } else {
            single_keys.push(item);
        }
    }
    let sort_counts = |left: &KeyCount, right: &KeyCount| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    };
    single_keys.sort_by(sort_counts);
    shortcuts.sort_by(sort_counts);

    Ok(KeyStatsSnapshot {
        source: "KeyStats 1.1.1 · 本机只读",
        updated_at,
        today: KeyStatsToday {
            date: legacy.today.date,
            key_strokes: legacy.today.key_strokes,
            left_clicks: legacy.today.left_clicks,
            right_clicks: legacy.today.right_clicks,
            mouse_distance: legacy.today.mouse_distance,
            scroll_distance: legacy.today.scroll_distance,
        },
        history: legacy
            .history
            .into_iter()
            .map(|item| KeyStatsHistoryDay {
                date: item.date,
                key_strokes: item.key_strokes,
                clicks: item.clicks,
                mouse_distance: item.mouse_distance,
                scroll_distance: item.scroll_distance,
            })
            .collect(),
        single_keys,
        shortcuts,
        capabilities: KeyStatsCapabilities {
            history_granularity: "day",
            minute_density: false,
            split_historical_clicks: false,
            sensitive_surface_exclusion: false,
            delete_by_date: false,
            timezone_semantics: "utc-date-bucket",
        },
    })
}

fn decode_contents(bytes: &[u8]) -> Result<String, KeyStatsReadError> {
    if let Some(rest) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return String::from_utf8(rest.to_vec()).map_err(|error| invalid_data(error.to_string()));
    }
    let (bytes, big_endian) = if let Some(rest) = bytes.strip_prefix(&[0xff, 0xfe]) {
        (rest, false)
    } else if let Some(rest) = bytes.strip_prefix(&[0xfe, 0xff]) {
        (rest, true)
    } else {
        return String::from_utf8(bytes.to_vec()).map_err(|error| invalid_data(error.to_string()));
    };
    if bytes.len() % 2 != 0 {
        return Err(invalid_data("KeyStats UTF-16 数据长度无效"));
    }
    let units = bytes.chunks_exact(2).map(|pair| {
        if big_endian {
            u16::from_be_bytes([pair[0], pair[1]])
        } else {
            u16::from_le_bytes([pair[0], pair[1]])
        }
    });
    String::from_utf16(&units.collect::<Vec<_>>()).map_err(|error| invalid_data(error.to_string()))
}

fn source_path() -> Result<PathBuf, KeyStatsReadError> {
    let app_data = std::env::var_os("APPDATA").ok_or_else(|| KeyStatsReadError {
        code: "not_found",
        message: "Windows APPDATA 路径不可用".into(),
    })?;
    Ok(PathBuf::from(app_data)
        .join("keystats")
        .join("keystats-data.json"))
}

fn modified_millis(path: &Path) -> u128 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_millis())
}

fn read_snapshot(path: &Path) -> Result<KeyStatsSnapshot, KeyStatsReadError> {
    let mut last_message = String::from("KeyStats 数据暂时不可读");
    for attempt in 0..READ_ATTEMPTS {
        match fs::read(path) {
            Ok(bytes) => match decode_contents(&bytes)
                .and_then(|contents| parse_snapshot(&contents, modified_millis(path)))
            {
                Ok(snapshot) => return Ok(snapshot),
                Err(error) => last_message = error.message,
            },
            Err(error) => last_message = error.to_string(),
        }
        if attempt + 1 < READ_ATTEMPTS {
            thread::sleep(RETRY_DELAY);
        }
    }
    Err(KeyStatsReadError {
        code: if path.is_file() {
            "read_failed"
        } else {
            "not_found"
        },
        message: last_message,
    })
}

#[tauri::command]
pub(crate) fn get_key_stats_snapshot() -> Result<KeyStatsSnapshot, KeyStatsReadError> {
    let path = source_path()?;
    read_snapshot(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_splits_real_key_categories() {
        let json = r#"{
          "today":{"date":"2026-07-16","keyStrokes":20,"leftClicks":4,"rightClicks":1,"mouseDistance":12.5,"scrollDistance":7},
          "history":[{"date":"2026-07-15","keyStrokes":10,"clicks":3,"mouseDistance":9.5,"scrollDistance":2}],
          "keyStats":{"Space":8,"Ctrl + C":3,"A":5,"Zero":0},
          "totalKeyStats":{}
        }"#;
        let snapshot = parse_snapshot(json, 123).expect("valid fixture");
        assert_eq!(snapshot.updated_at, 123);
        assert_eq!(snapshot.single_keys[0].key, "Space");
        assert_eq!(snapshot.shortcuts[0].key, "Ctrl + C");
        assert_eq!(snapshot.history.len(), 1);
    }

    #[test]
    fn rejects_non_iso_dates() {
        let json = r#"{
          "today":{"date":"16/07/2026","keyStrokes":0,"leftClicks":0,"rightClicks":0,"mouseDistance":0,"scrollDistance":0},
          "history":[],"keyStats":{},"totalKeyStats":{}
        }"#;
        assert!(parse_snapshot(json, 0).is_err());
    }

    #[test]
    fn rejects_impossible_calendar_dates() {
        assert!(!is_iso_date("2026-02-29"));
        assert!(is_iso_date("2024-02-29"));
        assert!(!is_iso_date("2026-13-01"));
    }

    #[test]
    fn decodes_utf8_bom_and_utf16_little_endian() {
        assert_eq!(decode_contents(b"\xef\xbb\xbf{}").unwrap(), "{}");
        let utf16 = [0xff, 0xfe, b'{', 0, b'}', 0];
        assert_eq!(decode_contents(&utf16).unwrap(), "{}");
    }
}
