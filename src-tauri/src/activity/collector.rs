use super::{
    capture::{capture_observation, CapturedObservation, IDLE_THRESHOLD_MILLIS},
    model::{
        ActivityObservation, ActivitySlice, CollectorHealth, DeviceState, SAMPLE_INTERVAL_SECONDS,
    },
    storage::{append_slice, append_slice_once},
};
use crate::icons::IconService;
use crate::reminders::ReminderService;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError, Sender, SyncSender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};

const MAX_CONTIGUOUS_MILLIS: u64 = SAMPLE_INTERVAL_SECONDS * 2 * 1_000;
/// Consecutive samples with an identical observation fold into one pending
/// interval; it is checkpointed to disk at least this often so an abnormal
/// exit loses at most one merge window of activity.
const MERGED_SLICE_MILLIS: u64 = 60_000;
const CONTROL_TIMEOUT: Duration = Duration::from_secs(2);
/// Slices whose disk write failed are retried from this in-memory queue
/// instead of being dropped — a disk-full or AV-lock outage is reported via
/// health.last_error without silently losing the observed activity.
const RETRY_QUEUE_CAPACITY: usize = 64;

/// (start, edge, observation, generation) — `edge` is the boundary of the last
/// sample folded into this pending interval, so a sampling gap or a wall-clock
/// rollback is detected by comparing the next boundary against `edge`, never by
/// measuring `end - start`.
type PendingActivity = (u64, u64, ActivityObservation, u64);

struct CaptureServices<'a> {
    health: &'a HealthState,
    icons: &'a IconService,
    reminders: &'a ReminderService,
    app: &'a tauri::AppHandle,
}

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

fn observation_boundary(
    previous: &Option<PendingActivity>,
    current: &CapturedObservation,
    now: u64,
) -> u64 {
    let Some((start, _, observation, _)) = previous else {
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
    // `clamp(min, max)` requires min <= max; when the wall clock rolls back
    // behind `start`, min(now, start) keeps the bounds ordered instead of
    // panicking the collector thread.
    now.saturating_sub(u64::from(excess))
        .clamp((*start).min(now), now)
}

/// What to do with the pending interval at the next sample boundary.
enum PendingAction {
    /// Same observation, same generation, no sampling gap — keep accumulating.
    Extend,
    /// Close the pending interval at `end`, then reopen at `open_at`.
    /// `open_at` is never before the last observed edge, so slices written
    /// after a clock rollback cannot overlap the ones already on disk.
    Reopen { end: u64, open_at: u64 },
}

fn pending_action(
    previous: &Option<PendingActivity>,
    current: &ActivityObservation,
    generation: u64,
    boundary: u64,
) -> PendingAction {
    let Some((_, edge, observation, pending_generation)) = previous.as_ref() else {
        return PendingAction::Reopen {
            end: boundary,
            open_at: boundary,
        };
    };
    let contiguous = boundary >= *edge && boundary - *edge <= MAX_CONTIGUOUS_MILLIS;
    if contiguous && *pending_generation == generation && *observation == *current {
        PendingAction::Extend
    } else if contiguous {
        PendingAction::Reopen {
            end: boundary,
            open_at: boundary,
        }
    } else {
        // Sampling gap (sleep, suspend, NTP step) or wall-clock rollback: keep
        // only the continuously observed portion and never reopen behind the
        // last edge, so on-disk slices stay disjoint.
        PendingAction::Reopen {
            end: *edge,
            open_at: boundary.max(*edge),
        }
    }
}

fn pending_slice(previous: &Option<PendingActivity>, end: u64) -> Option<ActivitySlice> {
    let (start, _, observation, generation) = previous.as_ref()?;
    (end > *start).then(|| ActivitySlice {
        version: 1,
        start: *start,
        end,
        generation: *generation,
        observation: observation.clone(),
    })
}

fn queue_failed_slice(slice: ActivitySlice, retry: &mut VecDeque<ActivitySlice>) {
    if retry.len() >= RETRY_QUEUE_CAPACITY {
        // Bounded: under a truly persistent outage the oldest queued slice is
        // sacrificed rather than letting memory grow without limit.
        retry.pop_front();
    }
    retry.push_back(slice);
}

/// Single-attempt drain — no per-slice backoff so a still-failing disk stalls
/// the collector for at most one write per queued slice.
fn drain_retry_queue(retry: &mut VecDeque<ActivitySlice>) {
    while let Some(slice) = retry.pop_front() {
        if append_slice_once(&slice).is_err() {
            retry.push_front(slice);
            break;
        }
    }
}

fn write_previous(
    previous: &mut Option<PendingActivity>,
    end: u64,
    health: &HealthState,
    retry: &mut VecDeque<ActivitySlice>,
) -> Result<(), String> {
    // Never write past the last observed edge: an explicit boundary (pause,
    // shutdown) may land between samples, but a long sampling gap must not be
    // recorded as continuous activity.
    let close_end = match previous.as_ref() {
        Some((_, edge, ..)) if end >= *edge && end - *edge <= MAX_CONTIGUOUS_MILLIS => end,
        Some((_, edge, ..)) => *edge,
        None => return Ok(()),
    };
    let Some(slice) = pending_slice(previous, close_end) else {
        *previous = None;
        return Ok(());
    };
    match append_slice(&slice) {
        Ok(()) => {
            *previous = None;
            health.last_write_at.store(close_end, Ordering::Release);
            // A successful write proves the disk is writable again; flush the
            // backlog now rather than waiting for the next tick.
            drain_retry_queue(retry);
            *health
                .last_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
            Ok(())
        }
        Err(error) => {
            // The pending interval moves into the retry queue instead of being
            // dropped or pinning `previous` — later samples reopen normally.
            queue_failed_slice(slice, retry);
            *previous = None;
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
    services: &CaptureServices<'_>,
    retry: &mut VecDeque<ActivitySlice>,
) -> Result<(), String> {
    let current = capture_observation();
    if let Some((identity, path)) = current.icon_hint.clone() {
        services
            .icons
            .register_executable_hint(services.app, identity, path);
    }
    services.reminders.observe(
        services.app,
        now,
        current.observation.device_state == DeviceState::Active,
    );
    let boundary = observation_boundary(previous, &current, now);
    match pending_action(previous, &current.observation, generation, boundary) {
        PendingAction::Extend => {
            if let Some((start, edge, _, _)) = previous.as_mut() {
                *edge = boundary;
                if *edge - *start >= MERGED_SLICE_MILLIS {
                    let checkpoint = *edge;
                    // The interval is queued (not lost) on write failure, so
                    // the merged window reopens at the checkpoint regardless.
                    let _ = write_previous(previous, checkpoint, services.health, retry);
                    *previous = Some((
                        checkpoint,
                        checkpoint,
                        current.observation.clone(),
                        generation,
                    ));
                }
            }
        }
        PendingAction::Reopen { end, open_at } => {
            let _ = write_previous(previous, end, services.health, retry);
            *previous = Some((open_at, open_at, current.observation, generation));
        }
    }
    Ok(())
}

fn send_reply(reply: SyncSender<Result<(), String>>, result: Result<(), String>) {
    let _ = reply.send(result);
}

/// Clears `health.running` on every exit path — including unwind from a panic
/// inside the loop — so `health()` never reports a dead worker as running.
struct RunningFlag<'a> {
    health: &'a HealthState,
}

impl Drop for RunningFlag<'_> {
    fn drop(&mut self) {
        self.health.running.store(false, Ordering::Release);
    }
}

fn run_loop(
    receiver: &mpsc::Receiver<CollectorCommand>,
    recording: bool,
    generation: u64,
    services: &CaptureServices<'_>,
) {
    let mut previous = None;
    let mut retry = VecDeque::new();
    let mut recording_now = recording;
    let mut current_generation = generation;

    if recording_now {
        if let Some(now) = unix_millis() {
            let _ = capture_sample(&mut previous, current_generation, now, services, &mut retry);
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
                            services,
                            &mut retry,
                        );
                    }
                }
                // Retry one queued backlog slice per tick even while paused —
                // a recovered disk should not wait for the next observation.
                drain_retry_queue(&mut retry);
            }
            Err(RecvTimeoutError::Disconnected) => {
                if recording_now {
                    if let Some(now) = unix_millis() {
                        let _ = write_previous(&mut previous, now, services.health, &mut retry);
                    }
                }
                drain_retry_queue(&mut retry);
                break;
            }
            Ok(CollectorCommand::SetRecording {
                recording,
                generation,
                at,
                reply,
            }) => {
                let saved_recording = recording_now;
                let saved_generation = current_generation;
                let result = if recording == recording_now && generation == current_generation {
                    Ok(())
                } else if recording {
                    previous = None;
                    capture_sample(&mut previous, generation, at, services, &mut retry).map(|()| {
                        recording_now = true;
                        current_generation = generation;
                    })
                } else {
                    write_previous(&mut previous, at, services.health, &mut retry).map(|()| {
                        recording_now = false;
                        current_generation = generation;
                        services.reminders.observe(services.app, at, false);
                    })
                };
                if reply.send(result).is_err() {
                    // The caller already timed out and rolled the shared state
                    // back; undo the application here so the worker cannot keep
                    // recording while the UI reports paused. Any slice already
                    // written stays on disk — it is real observed data.
                    recording_now = saved_recording;
                    current_generation = saved_generation;
                    previous = None;
                }
            }
            Ok(CollectorCommand::Shutdown { at, reply }) => {
                let result = if recording_now {
                    write_previous(&mut previous, at, services.health, &mut retry)
                } else {
                    Ok(())
                };
                // Give the backlog one last bounded flush before reporting.
                drain_retry_queue(&mut retry);
                services.reminders.observe(services.app, at, false);
                // Always stop: a slice that could not be flushed is queued in
                // memory and cannot outlive the process anyway, so a failing
                // disk must never trap the worker (and the app) open. The
                // caller still receives the real flush outcome.
                send_reply(reply, result);
                break;
            }
        }
    }
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
                let _running = RunningFlag {
                    health: &thread_health,
                };
                let services = CaptureServices {
                    health: &thread_health,
                    icons: &icons,
                    reminders: &reminders,
                    app: &app,
                };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_loop(&receiver, recording, generation, &services);
                }));
                if result.is_err() {
                    *thread_health
                        .last_error
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                        Some("活动采集线程异常终止".into());
                }
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
        // The worker always exits once it receives Shutdown, so join whenever
        // the reply arrived (or the thread already finished on its own).
        let should_join = acknowledged || worker.as_ref().is_some_and(JoinHandle::is_finished);
        if should_join {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        } else {
            // The command never reached the worker (dead channel or timeout);
            // it may still be running — allow a later shutdown to retry.
            self.stopped.store(false, Ordering::Release);
        }
        result
    }

    pub(crate) fn health(&self) -> CollectorHealth {
        let worker_done = self
            .worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .is_none_or(JoinHandle::is_finished);
        let mut running = self.health.running.load(Ordering::Acquire);
        if running && worker_done {
            // The worker ended without clearing the flag (or was already
            // joined); correct the flag once so health never lies about a dead
            // collector.
            running = false;
            self.health.running.store(false, Ordering::Release);
            let mut last_error = self
                .health
                .last_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if last_error.is_none() {
                *last_error = Some("活动采集线程已退出".into());
            }
        }
        let last_write = self.health.last_write_at.load(Ordering::Acquire);
        CollectorHealth {
            collector_running: running,
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

    fn idle_capture() -> CapturedObservation {
        CapturedObservation {
            observation: ActivityObservation {
                device_state: DeviceState::Idle,
                app_id: None,
                app_name: None,
                ai_tool: false,
            },
            idle_millis: Some(IDLE_THRESHOLD_MILLIS + 3_000),
            icon_hint: None,
        }
    }

    #[test]
    fn gaps_and_rollbacks_close_at_the_last_observed_edge() {
        let previous = Some((1_000, 1_000, observation(), 1));
        // Sampling gap (sleep/suspend): keep only the observed portion.
        match pending_action(&previous, &observation(), 2, 40_000) {
            PendingAction::Reopen { end, open_at } => {
                assert_eq!((end, open_at), (1_000, 40_000));
            }
            PendingAction::Extend => panic!("gap must reopen"),
        }
        // Wall-clock rollback: reopen at the edge so slices never overlap.
        match pending_action(&previous, &observation(), 1, 500) {
            PendingAction::Reopen { end, open_at } => {
                assert_eq!((end, open_at), (1_000, 1_000));
            }
            PendingAction::Extend => panic!("rollback must reopen"),
        }
        // Same observation inside the contiguous window just extends.
        assert!(matches!(
            pending_action(&previous, &observation(), 1, 11_000),
            PendingAction::Extend
        ));
        // Same observation but a new generation reopens at the boundary.
        match pending_action(&previous, &observation(), 9, 11_000) {
            PendingAction::Reopen { end, open_at } => {
                assert_eq!((end, open_at), (11_000, 11_000));
            }
            PendingAction::Extend => panic!("generation change must reopen"),
        }
    }

    #[test]
    fn write_never_extends_past_the_last_observed_edge() {
        // A shutdown arriving long after the last sample (sleep in between)
        // must not record the unobserved stretch.
        let health = HealthState {
            running: AtomicBool::new(true),
            last_write_at: AtomicU64::new(0),
            last_error: Mutex::new(None),
        };
        let mut previous = Some((1_000, 1_000, observation(), 1));
        let mut retry = VecDeque::new();
        write_previous(&mut previous, 60_000, &health, &mut retry).unwrap();
        // close_end clamps to the edge, which equals start → nothing to write.
        assert!(previous.is_none());
        assert!(retry.is_empty());
    }

    #[test]
    fn clock_rollback_boundary_does_not_panic() {
        let previous = Some((10_000, 10_000, observation(), 3));
        assert_eq!(
            observation_boundary(&previous, &idle_capture(), 5_000),
            5_000
        );
    }

    #[test]
    fn closes_active_interval_at_idle_threshold_boundary() {
        let previous = Some((1_000, 1_000, observation(), 3));
        assert_eq!(
            observation_boundary(&previous, &idle_capture(), 20_000),
            17_000
        );
    }

    #[test]
    fn pause_boundary_uses_command_timestamp_and_preserves_generation() {
        let previous = Some((10_000, 10_000, observation(), 7));
        let slice = pending_slice(&previous, 15_250).expect("closeable interval");
        assert_eq!((slice.start, slice.end), (10_000, 15_250));
        assert_eq!(slice.generation, 7);
    }

    #[test]
    fn rapid_transitions_produce_distinct_generation_slices() {
        let first = pending_slice(&Some((1_000, 1_000, observation(), 1)), 2_000).unwrap();
        let second = pending_slice(&Some((2_001, 2_001, observation(), 2)), 2_500).unwrap();
        assert_eq!(first.end, 2_000);
        assert_eq!(second.start, 2_001);
        assert_ne!(first.generation, second.generation);
    }
}
