use super::{
    capture::{capture_observation, CapturedObservation, IDLE_THRESHOLD_MILLIS},
    model::{
        ActivityObservation, ActivitySlice, CollectorHealth, DeviceState, SAMPLE_INTERVAL_SECONDS,
    },
    storage::append_slice,
};
use crate::icons::IconService;
use crate::reminders::ReminderService;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError, Sender, SyncSender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};

const MAX_CONTIGUOUS_MILLIS: u64 = SAMPLE_INTERVAL_SECONDS * 2 * 1_000;
const CONTROL_TIMEOUT: Duration = Duration::from_secs(2);

type PendingActivity = (u64, ActivityObservation, u64);

struct HealthState {
    running: AtomicBool,
    last_write_at: AtomicU64,
    last_error: Mutex<Option<String>>,
}

enum CollectorCommand {
    SetRecording {
        recording: bool,
        generation: u64,
        at: u64,
        reply: SyncSender<Result<(), String>>,
    },
    Shutdown {
        at: u64,
        reply: SyncSender<Result<(), String>>,
    },
}

pub(crate) struct ActivityCollector {
    health: Arc<HealthState>,
    control: Sender<CollectorCommand>,
    stopped: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
}

fn unix_millis() -> Option<u64> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

fn should_close_interval(start: u64, end: u64) -> bool {
    end > start && end - start <= MAX_CONTIGUOUS_MILLIS
}

fn observation_boundary(
    previous: &Option<PendingActivity>,
    current: &CapturedObservation,
    now: u64,
) -> u64 {
    let Some((start, observation, _)) = previous else {
        return now;
    };
    if observation.device_state != DeviceState::Active
        || current.observation.device_state != DeviceState::Idle
    {
        return now;
    }
    let excess = current
        .idle_millis
        .unwrap_or(IDLE_THRESHOLD_MILLIS)
        .saturating_sub(IDLE_THRESHOLD_MILLIS);
    now.saturating_sub(u64::from(excess)).clamp(*start, now)
}

fn pending_slice(previous: &Option<PendingActivity>, end: u64) -> Option<ActivitySlice> {
    let (start, observation, generation) = previous.as_ref()?;
    should_close_interval(*start, end).then(|| ActivitySlice {
        version: 1,
        start: *start,
        end,
        generation: *generation,
        observation: observation.clone(),
    })
}

fn write_previous(
    previous: &mut Option<PendingActivity>,
    end: u64,
    health: &HealthState,
) -> Result<(), String> {
    let Some(slice) = pending_slice(previous, end) else {
        *previous = None;
        return Ok(());
    };
    match append_slice(&slice) {
        Ok(()) => {
            *previous = None;
            health.last_write_at.store(end, Ordering::Release);
            *health
                .last_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
            Ok(())
        }
        Err(error) => {
            *health
                .last_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(error.message.clone());
            Err(error.message)
        }
    }
}

fn capture_sample(
    previous: &mut Option<PendingActivity>,
    generation: u64,
    now: u64,
    health: &HealthState,
    icons: &IconService,
    reminders: &ReminderService,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    let current = capture_observation();
    if let Some((identity, path)) = current.icon_hint.clone() {
        icons.register_executable_hint(app, identity, path);
    }
    reminders.observe(
        app,
        now,
        current.observation.device_state == DeviceState::Active,
    );
    let boundary = observation_boundary(previous, &current, now);
    write_previous(previous, boundary, health)?;
    *previous = Some((boundary, current.observation, generation));
    Ok(())
}

fn send_reply(reply: SyncSender<Result<(), String>>, result: Result<(), String>) {
    let _ = reply.send(result);
}

impl ActivityCollector {
    pub(crate) fn start(
        recording: bool,
        generation: u64,
        icons: IconService,
        reminders: ReminderService,
        app: tauri::AppHandle,
    ) -> Self {
        let health = Arc::new(HealthState {
            running: AtomicBool::new(false),
            last_write_at: AtomicU64::new(0),
            last_error: Mutex::new(None),
        });
        let thread_health = health.clone();
        let (control, receiver) = mpsc::channel();
        let spawn_result = thread::Builder::new()
            .name("itime-activity-collector".into())
            .spawn(move || {
                thread_health.running.store(true, Ordering::Release);
                let mut previous = None;
                let mut recording_now = recording;
                let mut current_generation = generation;

                if recording_now {
                    if let Some(now) = unix_millis() {
                        let _ = capture_sample(
                            &mut previous,
                            current_generation,
                            now,
                            &thread_health,
                            &icons,
                            &reminders,
                            &app,
                        );
                    }
                }

                loop {
                    match receiver.recv_timeout(Duration::from_secs(SAMPLE_INTERVAL_SECONDS)) {
                        Err(RecvTimeoutError::Timeout) => {
                            if recording_now {
                                if let Some(now) = unix_millis() {
                                    let _ = capture_sample(
                                        &mut previous,
                                        current_generation,
                                        now,
                                        &thread_health,
                                        &icons,
                                        &reminders,
                                        &app,
                                    );
                                }
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            if recording_now {
                                if let Some(now) = unix_millis() {
                                    let _ = write_previous(&mut previous, now, &thread_health);
                                }
                            }
                            break;
                        }
                        Ok(CollectorCommand::SetRecording {
                            recording,
                            generation,
                            at,
                            reply,
                        }) => {
                            let result =
                                if recording == recording_now && generation == current_generation {
                                    Ok(())
                                } else if recording {
                                    previous = None;
                                    capture_sample(
                                        &mut previous,
                                        generation,
                                        at,
                                        &thread_health,
                                        &icons,
                                        &reminders,
                                        &app,
                                    )
                                    .map(|()| {
                                        recording_now = true;
                                        current_generation = generation;
                                    })
                                } else {
                                    write_previous(&mut previous, at, &thread_health).map(|()| {
                                        recording_now = false;
                                        current_generation = generation;
                                        reminders.observe(&app, at, false);
                                    })
                                };
                            send_reply(reply, result);
                        }
                        Ok(CollectorCommand::Shutdown { at, reply }) => {
                            let result = if recording_now {
                                write_previous(&mut previous, at, &thread_health)
                            } else {
                                Ok(())
                            };
                            let should_stop = result.is_ok();
                            if should_stop {
                                reminders.observe(&app, at, false);
                            }
                            send_reply(reply, result);
                            if should_stop {
                                break;
                            }
                        }
                    }
                }
                thread_health.running.store(false, Ordering::Release);
            });
        let worker = match spawn_result {
            Ok(handle) => Some(handle),
            Err(error) => {
                *health
                    .last_error
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(error.to_string());
                None
            }
        };
        Self {
            health,
            control,
            stopped: AtomicBool::new(false),
            worker: Mutex::new(worker),
        }
    }

    pub(crate) fn set_recording(
        &self,
        recording: bool,
        generation: u64,
        at: u64,
    ) -> Result<(), String> {
        if self.stopped.load(Ordering::Acquire) {
            return Err("活动采集器已经停止".into());
        }
        let (reply, response) = mpsc::sync_channel(1);
        self.control
            .send(CollectorCommand::SetRecording {
                recording,
                generation,
                at,
                reply,
            })
            .map_err(|_| "活动采集控制通道不可用".to_string())?;
        response
            .recv_timeout(CONTROL_TIMEOUT)
            .map_err(|_| "活动采集器未及时确认状态切换".to_string())?
    }

    pub(crate) fn shutdown(&self, at: u64) -> Result<(), String> {
        if self.stopped.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let (reply, response) = mpsc::sync_channel(1);
        let (result, acknowledged) =
            match self.control.send(CollectorCommand::Shutdown { at, reply }) {
                Err(_) => (Err("活动采集控制通道不可用".to_string()), false),
                Ok(()) => match response.recv_timeout(CONTROL_TIMEOUT) {
                    Ok(result) => (result, true),
                    Err(_) => (Err("活动采集器未及时完成退出刷新".to_string()), false),
                },
            };
        let mut worker = self
            .worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let should_join = result.is_ok()
            && (acknowledged || worker.as_ref().is_some_and(JoinHandle::is_finished));
        if should_join {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        } else if result.is_err() {
            self.stopped.store(false, Ordering::Release);
        }
        result
    }

    pub(crate) fn health(&self) -> CollectorHealth {
        let last_write = self.health.last_write_at.load(Ordering::Acquire);
        CollectorHealth {
            collector_running: self.health.running.load(Ordering::Acquire),
            last_write_at: (last_write > 0).then_some(last_write),
            last_error: self
                .health
                .last_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
        }
    }
}

impl Drop for ActivityCollector {
    fn drop(&mut self) {
        let _ = self.shutdown(unix_millis().unwrap_or(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> ActivityObservation {
        ActivityObservation {
            device_state: DeviceState::Active,
            app_id: Some("code".into()),
            app_name: Some("Code".into()),
            ai_tool: false,
        }
    }

    #[test]
    fn rejects_sleep_or_suspend_sized_gaps() {
        assert!(should_close_interval(1_000, 11_000));
        assert!(!should_close_interval(1_000, 31_001));
        assert!(!should_close_interval(2_000, 1_000));
    }

    #[test]
    fn closes_active_interval_at_idle_threshold_boundary() {
        let previous = Some((1_000, observation(), 3));
        let current = CapturedObservation {
            observation: ActivityObservation {
                device_state: DeviceState::Idle,
                app_id: None,
                app_name: None,
                ai_tool: false,
            },
            idle_millis: Some(IDLE_THRESHOLD_MILLIS + 3_000),
            icon_hint: None,
        };
        assert_eq!(observation_boundary(&previous, &current, 20_000), 17_000);
    }

    #[test]
    fn pause_boundary_uses_command_timestamp_and_preserves_generation() {
        let previous = Some((10_000, observation(), 7));
        let slice = pending_slice(&previous, 15_250).expect("closeable interval");
        assert_eq!((slice.start, slice.end), (10_000, 15_250));
        assert_eq!(slice.generation, 7);
    }

    #[test]
    fn rapid_transitions_produce_distinct_generation_slices() {
        let first = pending_slice(&Some((1_000, observation(), 1)), 2_000).unwrap();
        let second = pending_slice(&Some((2_001, observation(), 2)), 2_500).unwrap();
        assert_eq!(first.end, 2_000);
        assert_eq!(second.start, 2_001);
        assert_ne!(first.generation, second.generation);
    }
}
