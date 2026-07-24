use chrono::{Local, Timelike};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

const MIN_INTERVAL_MINUTES: u64 = 10;
const MAX_INTERVAL_MINUTES: u64 = 240;
const MILLIS_PER_MINUTE: u64 = 60_000;

#[derive(Clone, Debug, PartialEq)]
struct ReminderConfig {
    enabled: bool,
    interval_minutes: u64,
    quiet_start: u16,
    quiet_end: u16,
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 50,
            quiet_start: 22 * 60,
            quiet_end: 8 * 60,
        }
    }
}

#[derive(Default)]
struct ReminderRuntime {
    config: ReminderConfig,
    active_since: Option<u64>,
    delivered_occurrence: u64,
}

impl ReminderRuntime {
    fn configure(&mut self, config: ReminderConfig) {
        if self.config == config {
            return;
        }
        self.config = config;
        self.active_since = None;
        self.delivered_occurrence = 0;
    }

    fn observe(&mut self, now: u64, active: bool, local_minute: u16) -> Option<u64> {
        if !self.config.enabled || !active {
            self.active_since = None;
            self.delivered_occurrence = 0;
            return None;
        }

        let active_since = *self.active_since.get_or_insert(now);
        let interval_millis = self
            .config
            .interval_minutes
            .saturating_mul(MILLIS_PER_MINUTE);
        let occurrence = now.saturating_sub(active_since) / interval_millis;
        if occurrence == 0
            || occurrence <= self.delivered_occurrence
            || within_quiet_hours(local_minute, self.config.quiet_start, self.config.quiet_end)
        {
            return None;
        }

        self.delivered_occurrence = occurrence;
        Some(occurrence.saturating_mul(self.config.interval_minutes))
    }
}

#[derive(Clone, Default)]
pub(crate) struct ReminderService {
    runtime: Arc<Mutex<ReminderRuntime>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReminderDueEvent {
    continuous_minutes: u64,
}

impl ReminderService {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn configure(
        &self,
        enabled: bool,
        interval_minutes: u64,
        quiet_start: &str,
        quiet_end: &str,
    ) -> Result<(), String> {
        if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&interval_minutes) {
            return Err(format!(
                "连续使用提醒需在 {MIN_INTERVAL_MINUTES}—{MAX_INTERVAL_MINUTES} 分钟之间"
            ));
        }
        let config = ReminderConfig {
            enabled,
            interval_minutes,
            quiet_start: parse_clock(quiet_start)?,
            quiet_end: parse_clock(quiet_end)?,
        };
        self.runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .configure(config);
        Ok(())
    }

    pub(crate) fn observe(&self, app: &AppHandle, now: u64, active: bool) {
        let local = Local::now();
        let minute = (local.hour() * 60 + local.minute()) as u16;
        let due = self
            .runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .observe(now, active, minute);
        let Some(continuous_minutes) = due else {
            return;
        };

        let body = format!(
            "你已连续使用电脑 {continuous_minutes} 分钟。起身活动一下、看看远处，再回来继续。"
        );
        match app
            .notification()
            .builder()
            .title("iTime · 休息一下")
            .body(body)
            .show()
        {
            Ok(()) => {
                let _ = app.emit("rest-reminder-due", ReminderDueEvent { continuous_minutes });
            }
            Err(error) => {
                let _ = app.emit("rest-reminder-error", error.to_string());
            }
        }
    }
}

fn parse_clock(value: &str) -> Result<u16, String> {
    let (hour, minute) = value
        .split_once(':')
        .ok_or_else(|| "安静时段格式无效".to_string())?;
    let hour = hour
        .parse::<u16>()
        .map_err(|_| "安静时段小时无效".to_string())?;
    let minute = minute
        .parse::<u16>()
        .map_err(|_| "安静时段分钟无效".to_string())?;
    if hour > 23 || minute > 59 {
        return Err("安静时段必须是 00:00—23:59".into());
    }
    Ok(hour * 60 + minute)
}

fn within_quiet_hours(current: u16, start: u16, end: u16) -> bool {
    start == end
        || if start < end {
            current >= start && current < end
        } else {
            current >= start || current < end
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured_runtime(interval_minutes: u64) -> ReminderRuntime {
        let mut runtime = ReminderRuntime::default();
        runtime.configure(ReminderConfig {
            enabled: true,
            interval_minutes,
            quiet_start: 22 * 60,
            quiet_end: 8 * 60,
        });
        runtime
    }

    #[test]
    fn emits_each_continuous_use_occurrence_once() {
        let mut runtime = configured_runtime(30);
        assert_eq!(runtime.observe(1_000, true, 12 * 60), None);
        assert_eq!(
            runtime.observe(1_000 + 30 * MILLIS_PER_MINUTE, true, 12 * 60),
            Some(30)
        );
        assert_eq!(
            runtime.observe(1_000 + 31 * MILLIS_PER_MINUTE, true, 12 * 60),
            None
        );
        assert_eq!(
            runtime.observe(1_000 + 60 * MILLIS_PER_MINUTE, true, 12 * 60),
            Some(60)
        );
    }

    #[test]
    fn inactivity_resets_the_continuous_session() {
        let mut runtime = configured_runtime(30);
        assert_eq!(runtime.observe(1_000, true, 12 * 60), None);
        assert_eq!(runtime.observe(2_000, false, 12 * 60), None);
        assert_eq!(
            runtime.observe(1_000 + 31 * MILLIS_PER_MINUTE, true, 12 * 60),
            None
        );
    }

    #[test]
    fn quiet_hours_delay_without_replaying_multiple_notifications() {
        let mut runtime = configured_runtime(30);
        assert_eq!(runtime.observe(1_000, true, 21 * 60), None);
        assert_eq!(
            runtime.observe(1_000 + 30 * MILLIS_PER_MINUTE, true, 22 * 60),
            None
        );
        assert_eq!(
            runtime.observe(1_000 + 60 * MILLIS_PER_MINUTE, true, 8 * 60),
            Some(60)
        );
        assert_eq!(
            runtime.observe(1_000 + 61 * MILLIS_PER_MINUTE, true, 8 * 60),
            None
        );
    }

    #[test]
    fn parses_and_checks_cross_midnight_quiet_hours() {
        assert_eq!(parse_clock("22:30"), Ok(1_350));
        assert!(within_quiet_hours(23 * 60, 22 * 60, 8 * 60));
        assert!(within_quiet_hours(7 * 60, 22 * 60, 8 * 60));
        assert!(!within_quiet_hours(12 * 60, 22 * 60, 8 * 60));
    }
}
