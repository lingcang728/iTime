use super::{
    capture::capture_observation,
    model::{ActivitySlice, CollectorHealth, SAMPLE_INTERVAL_SECONDS},
    storage::append_slice,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError, Sender},
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
    pub(crate) fn start(recording: Arc<AtomicBool>, generation: Arc<AtomicU64>) -> Self {
        let health = Arc::new(HealthState {
            running: AtomicBool::new(false),
            last_write_at: AtomicU64::new(0),
            last_error: Mutex::new(None),
        });
        let thread_health = health.clone();
        let (stop, receiver) = mpsc::channel();
        let spawn_result = thread::Builder::new()
            .name("itime-activity-collector".into())
            .spawn(move || {
                thread_health.running.store(true, Ordering::Release);
                let mut previous = None;
                let mut observed_generation = generation.load(Ordering::Acquire);
                loop {
                    let current_generation = generation.load(Ordering::Acquire);
                    if current_generation != observed_generation {
                        previous = None;
                        observed_generation = current_generation;
                    }
                    if recording.load(Ordering::Acquire) {
                        if let Some(now) = unix_millis() {
                            let current = capture_observation();
                            write_previous(&mut previous, now, &thread_health);
                            previous = Some((now, current));
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
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(handle) = worker.take() {
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
}
