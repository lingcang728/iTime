use super::{
    capture::{capture_observation, CapturedObservation, IDLE_THRESHOLD_MILLIS},
    model::{ActivitySlice, CollectorHealth, DeviceState, SAMPLE_INTERVAL_SECONDS},
    storage::append_slice,
};
use crate::icons::IconService;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};

const MAX_CONTIGUOUS_MILLIS: u64 = SAMPLE_INTERVAL_SECONDS * 2 * 1_000;

struct HealthState {
    running: AtomicBool,
    last_write_at: AtomicU64,
    last_error: Mutex<Option<String>>,
}

pub(crate) struct ActivityCollector {
    health: Arc<HealthState>,
    stop: Sender<()>,
    done: Mutex<Receiver<()>>,
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
    previous: &Option<(u64, super::model::ActivityObservation)>,
    current: &CapturedObservation,
    now: u64,
) -> u64 {
    let Some((start, observation)) = previous else {
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

fn write_previous(
    previous: &mut Option<(u64, super::model::ActivityObservation)>,
    end: u64,
    health: &HealthState,
) {
    let Some((start, observation)) = previous.take() else {
        return;
    };
    if !should_close_interval(start, end) {
        return;
    }
    let result = append_slice(&ActivitySlice {
        version: 1,
        start,
        end,
        observation,
    });
    match result {
        Ok(()) => {
            health.last_write_at.store(end, Ordering::Release);
            if let Ok(mut error) = health.last_error.lock() {
                *error = None;
            }
        }
        Err(error) => {
            if let Ok(mut value) = health.last_error.lock() {
                *value = Some(error.message);
            }
        }
    }
}

impl ActivityCollector {
    pub(crate) fn start(
        recording: Arc<AtomicBool>,
        generation: Arc<AtomicU64>,
        icons: IconService,
    ) -> Self {
        let health = Arc::new(HealthState {
            running: AtomicBool::new(false),
            last_write_at: AtomicU64::new(0),
            last_error: Mutex::new(None),
        });
        let thread_health = health.clone();
        let (stop, receiver) = mpsc::channel();
        let (done_sender, done_receiver) = mpsc::channel();
        let spawn_result = thread::Builder::new()
            .name("itime-activity-collector".into())
            .spawn(move || {
                thread_health.running.store(true, Ordering::Release);
                let mut previous = None;
                let mut observed_generation = generation.load(Ordering::Acquire);
                loop {
                    let current_generation = generation.load(Ordering::Acquire);
                    if current_generation != observed_generation {
                        if let Some(now) = unix_millis() {
                            write_previous(&mut previous, now, &thread_health);
                        } else {
                            previous = None;
                        }
                        observed_generation = current_generation;
                    }
                    if recording.load(Ordering::Acquire) {
                        if let Some(now) = unix_millis() {
                            let current = capture_observation();
                            if let Some((identity, path)) = current.icon_hint.clone() {
                                icons.register_executable_hint(identity, path);
                            }
                            let boundary = observation_boundary(&previous, &current, now);
                            write_previous(&mut previous, boundary, &thread_health);
                            previous = Some((boundary, current.observation));
                        }
                    } else {
                        previous = None;
                    }
                    match receiver.recv_timeout(Duration::from_secs(SAMPLE_INTERVAL_SECONDS)) {
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) | Ok(()) => {
                            if recording.load(Ordering::Acquire) {
                                if let Some(now) = unix_millis() {
                                    write_previous(&mut previous, now, &thread_health);
                                }
                            }
                            thread_health.running.store(false, Ordering::Release);
                            break;
                        }
                    }
                }
                let _ = done_sender.send(());
            });
        let worker = match spawn_result {
            Ok(handle) => Some(handle),
            Err(error) => {
                if let Ok(mut value) = health.last_error.lock() {
                    *value = Some(error.to_string());
                }
                None
            }
        };
        Self {
            health,
            stop,
            done: Mutex::new(done_receiver),
            worker: Mutex::new(worker),
        }
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
                .ok()
                .and_then(|value| value.clone()),
        }
    }
}

impl Drop for ActivityCollector {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        let finished = self
            .done
            .lock()
            .ok()
            .is_some_and(|done| done.recv_timeout(Duration::from_secs(2)).is_ok());
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(handle) = worker.take().filter(|_| finished) {
                let _ = handle.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_sleep_or_suspend_sized_gaps() {
        assert!(should_close_interval(1_000, 11_000));
        assert!(!should_close_interval(1_000, 31_001));
        assert!(!should_close_interval(2_000, 1_000));
    }

    #[test]
    fn closes_active_interval_at_idle_threshold_boundary() {
        let previous = Some((
            1_000,
            super::super::model::ActivityObservation {
                device_state: DeviceState::Active,
                app_id: None,
                app_name: None,
                ai_tool: false,
            },
        ));
        let current = CapturedObservation {
            observation: super::super::model::ActivityObservation {
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
}
