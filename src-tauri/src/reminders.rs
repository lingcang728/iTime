use chrono::{Local, Timelike};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

const MIN_INTERVAL_MINUTES: u64 = 10;
const MAX_INTERVAL_MINUTES: u64 = 240;
const MILLIS_PER_MINUTE: u64 = 60_000;
/// 与采集层 MAX_CONTIGUOUS_MILLIS（2×10s 采样间隔）对齐：两次 observe 间隔
/// 超过该值视为休眠/冻结，连续使用会话从头算起，不把睡眠时长计进去。
const OBSERVE_GAP_RESET_MILLIS: u64 = 20_000;

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
    last_observe_at: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReminderDueEvent {
    occurrence_id: String,
    continuous_minutes: u64,
}

/// 当前生效的提醒配置快照。提醒配置的权威副本在前端 localStorage，
/// 该快照让前端可以核对「UI 显示的状态」与「实际生效的配置」是否一致
/// （启动重推 IPC 失败时两侧会静默分叉）。
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReminderConfigSnapshot {
    pub(crate) enabled: bool,
    pub(crate) interval_minutes: u64,
    /// "HH:MM" 本地时间，与 configure_reminders 入参格式一致。
    pub(crate) quiet_start: String,
    pub(crate) quiet_end: String,
}

impl ReminderRuntime {
    fn configure(&mut self, config: ReminderConfig) {
        if self.config == config {
            return;
        }
        self.config = config;
        self.active_since = None;
        self.delivered_occurrence = 0;
        self.last_observe_at = None;
    }

    fn observe(&mut self, now: u64, active: bool, local_minute: u16) -> Option<ReminderDueEvent> {
        // 休眠/冻结期间没有 observe 发生，唤醒后的第一条 observe 间隔会很大。
        // 此时重置会话起点，避免「已连续使用」把睡眠时长算进去。
        let resumed_from_gap = self
            .last_observe_at
            .is_some_and(|last| now.saturating_sub(last) > OBSERVE_GAP_RESET_MILLIS);
        self.last_observe_at = Some(now);
        if resumed_from_gap {
            self.active_since = None;
            self.delivered_occurrence = 0;
        }

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
        Some(ReminderDueEvent {
            occurrence_id: format!("{active_since}:{interval_millis}:{occurrence}"),
            continuous_minutes: occurrence.saturating_mul(self.config.interval_minutes),
        })
    }
}

#[derive(Clone, Default)]
pub(crate) struct ReminderService {
    runtime: Arc<Mutex<ReminderRuntime>>,
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

    pub(crate) fn config_snapshot(&self) -> ReminderConfigSnapshot {
        let config = self
            .runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .config
            .clone();
        ReminderConfigSnapshot {
            enabled: config.enabled,
            interval_minutes: config.interval_minutes,
            quiet_start: format_clock(config.quiet_start),
            quiet_end: format_clock(config.quiet_end),
        }
    }

    pub(crate) fn observe(&self, app: &AppHandle, now: u64, active: bool) {
        let local = Local::now();
        let minute = (local.hour() * 60 + local.minute()) as u16;
        let due = self
            .runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .observe(now, active, minute);
        let Some(occurrence) = due else {
            return;
        };

        let body = format!(
            "你已连续使用电脑 {} 分钟。起身活动一下、看看远处，再回来继续。",
            occurrence.continuous_minutes
        );
        let _ = app.emit("rest-reminder-due", occurrence);
        if let Err(error) = app
            .notification()
            .builder()
            .title("iTime · 休息一下")
            .body(body)
            .show()
        {
            let _ = app.emit("rest-reminder-error", error.to_string());
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

fn format_clock(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn within_quiet_hours(current: u16, start: u16, end: u16) -> bool {
    if start == end {
        // 起止相同表示「未设置安静时段」，而不是全天安静——前端已禁止相等值，
        // 旁路写入的相等值不应静默吞掉全部提醒。
        return false;
    }
    if start < end {
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

    /// 按真实采样节奏（每 10s 一次 observe）走完一段时间；大跳跃等价于休眠。
    fn observe_continuous(
        runtime: &mut ReminderRuntime,
        from: u64,
        to: u64,
        minute: u16,
    ) -> Option<ReminderDueEvent> {
        let mut due = None;
        let mut now = from;
        while now <= to {
            if let Some(event) = runtime.observe(now, true, minute) {
                due = Some(event);
            }
            now += 10_000;
        }
        due
    }

    #[test]
    fn emits_each_continuous_use_occurrence_once() {
        let mut runtime = configured_runtime(30);
        assert_eq!(runtime.observe(1_000, true, 12 * 60), None);
        assert_eq!(
            observe_continuous(
                &mut runtime,
                11_000,
                1_000 + 30 * MILLIS_PER_MINUTE,
                12 * 60
            )
            .map(|event| event.continuous_minutes),
            Some(30)
        );
        assert_eq!(
            observe_continuous(
                &mut runtime,
                1_000 + 30 * MILLIS_PER_MINUTE + 10_000,
                1_000 + 59 * MILLIS_PER_MINUTE,
                12 * 60
            ),
            None
        );
        assert_eq!(
            observe_continuous(
                &mut runtime,
                1_000 + 59 * MILLIS_PER_MINUTE + 10_000,
                1_000 + 60 * MILLIS_PER_MINUTE,
                12 * 60
            )
            .map(|event| event.continuous_minutes),
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
        // 安静时段（22:00—08:00）内持续活跃到 60 分钟：所有档位都被压掉。
        assert_eq!(
            observe_continuous(
                &mut runtime,
                11_000,
                1_000 + 60 * MILLIS_PER_MINUTE,
                22 * 60
            ),
            None
        );
        // 安静结束后第一次采样补发当前档一次，之后不重复。
        assert_eq!(
            runtime
                .observe(1_000 + 60 * MILLIS_PER_MINUTE + 10_000, true, 8 * 60)
                .map(|event| event.continuous_minutes),
            Some(60)
        );
        assert_eq!(
            runtime.observe(1_000 + 60 * MILLIS_PER_MINUTE + 20_000, true, 8 * 60),
            None
        );
    }

    #[test]
    fn sleep_gap_restarts_the_continuous_session() {
        let mut runtime = configured_runtime(30);
        assert_eq!(runtime.observe(1_000, true, 12 * 60), None);
        // 休眠导致两次 observe 间隔超过阈值：会话重置，不补弹旧档位。
        let wake = 1_000 + 30 * MILLIS_PER_MINUTE;
        assert_eq!(runtime.observe(wake, true, 12 * 60), None);
        assert_eq!(
            observe_continuous(
                &mut runtime,
                wake + 10_000,
                wake + 30 * MILLIS_PER_MINUTE,
                12 * 60
            )
            .map(|event| event.continuous_minutes),
            Some(30)
        );
    }

    #[test]
    fn parses_and_checks_cross_midnight_quiet_hours() {
        assert_eq!(parse_clock("22:30"), Ok(1_350));
        assert!(within_quiet_hours(23 * 60, 22 * 60, 8 * 60));
        assert!(within_quiet_hours(7 * 60, 22 * 60, 8 * 60));
        assert!(!within_quiet_hours(12 * 60, 22 * 60, 8 * 60));
    }

    #[test]
    fn equal_quiet_bounds_mean_no_quiet_hours() {
        for minute in [0, 12 * 60, 22 * 60, 23 * 60] {
            assert!(!within_quiet_hours(minute, 22 * 60, 22 * 60));
            assert!(!within_quiet_hours(minute, 0, 0));
        }
    }
}
