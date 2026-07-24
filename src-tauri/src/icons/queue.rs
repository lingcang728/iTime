use super::extract::{extract_and_cache, try_cache_hit, ExtractRequest, IconSource};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const MAX_INFLIGHT: usize = 3;
const MAX_QUEUED: usize = 256;
const MAX_FAILURES: usize = 512;
const MAX_PATH_HINTS: usize = 512;
const FAILURE_COOLDOWN: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconUpdateEvent {
    pub app_identity: String,
    pub status: String,
    pub cache_path: Option<String>,
    pub icon_source: String,
    pub width: u32,
    pub height: u32,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone)]
struct FailureRecord {
    until: Instant,
    code: String,
}

#[derive(Clone)]
pub struct IconService {
    inner: Arc<Mutex<IconServiceInner>>,
}

struct IconServiceInner {
    inflight: HashSet<String>,
    queued: VecDeque<PendingJob>,
    failures: HashMap<String, FailureRecord>,
    active: usize,
    path_hints: HashMap<String, PathBuf>,
    hint_order: VecDeque<String>,
}

struct PendingJob {
    app: AppHandle,
    req: ExtractRequest,
}

impl IconService {
    pub(crate) fn register_executable_hint(
        &self,
        app: &AppHandle,
        app_identity: String,
        path: PathBuf,
    ) {
        if !path.is_file() {
            return;
        }
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let changed = guard
            .path_hints
            .get(&app_identity)
            .is_none_or(|current| current != &path);
        let failed = guard.failures.remove(&app_identity).is_some();
        guard
            .hint_order
            .retain(|identity| identity != &app_identity);
        guard.hint_order.push_back(app_identity.clone());
        guard.path_hints.insert(app_identity.clone(), path);
        while guard.hint_order.len() > MAX_PATH_HINTS {
            if let Some(oldest) = guard.hint_order.pop_front() {
                guard.path_hints.remove(&oldest);
            }
        }
        drop(guard);
        if changed || failed {
            let _ = app.emit("app-icon-hint-updated", app_identity);
        }
    }

    fn apply_path_hint(&self, req: &mut ExtractRequest) {
        if req.executable_path.is_some() {
            return;
        }
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        req.executable_path = guard
            .path_hints
            .get(&req.app_identity)
            .map(|path| path.to_string_lossy().to_string());
    }

    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(IconServiceInner {
                inflight: HashSet::new(),
                queued: VecDeque::new(),
                failures: HashMap::new(),
                active: 0,
                path_hints: HashMap::new(),
                hint_order: VecDeque::new(),
            })),
        }
    }

    /// Cache-only fast path, otherwise enqueue background extraction (never blocks on Shell).
    pub fn try_get_cached_or_enqueue(
        &self,
        app: AppHandle,
        mut req: ExtractRequest,
    ) -> IconUpdateEvent {
        self.apply_path_hint(&mut req);
        if let Some(hit) = try_cache_hit(&req) {
            return IconUpdateEvent {
                app_identity: req.app_identity,
                status: "resolved".into(),
                cache_path: Some(hit.png_path.to_string_lossy().to_string()),
                icon_source: hit.source.as_str().into(),
                width: hit.width,
                height: hit.height,
                error_code: None,
            };
        }

        let identity = req.app_identity.clone();
        let size = req.size;

        {
            let mut guard = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard
                .failures
                .retain(|_, failure| failure.until > Instant::now());

            if let Some(failure) = guard.failures.get(&identity) {
                if failure.until > Instant::now() {
                    return IconUpdateEvent {
                        app_identity: identity,
                        status: "failed".into(),
                        cache_path: None,
                        icon_source: IconSource::Fallback.as_str().into(),
                        width: size,
                        height: size,
                        error_code: Some(failure.code.clone()),
                    };
                }
                guard.failures.remove(&identity);
            }

            if guard.inflight.contains(&identity) {
                return IconUpdateEvent {
                    app_identity: identity,
                    status: "loading".into(),
                    cache_path: None,
                    icon_source: IconSource::Fallback.as_str().into(),
                    width: size,
                    height: size,
                    error_code: None,
                };
            }

            if guard.queued.len() >= MAX_QUEUED {
                return IconUpdateEvent {
                    app_identity: identity,
                    status: "failed".into(),
                    cache_path: None,
                    icon_source: IconSource::Fallback.as_str().into(),
                    width: size,
                    height: size,
                    error_code: Some("icon_queue_full".into()),
                };
            }

            guard.inflight.insert(identity.clone());
            guard.queued.push_back(PendingJob { app, req });
            Self::pump_locked(&mut guard, Arc::clone(&self.inner));
        }

        IconUpdateEvent {
            app_identity: identity,
            status: "loading".into(),
            cache_path: None,
            icon_source: IconSource::Fallback.as_str().into(),
            width: size,
            height: size,
            error_code: None,
        }
    }

    fn pump_locked(guard: &mut IconServiceInner, service: Arc<Mutex<IconServiceInner>>) {
        while guard.active < MAX_INFLIGHT {
            let Some(job) = guard.queued.pop_front() else {
                break;
            };
            guard.active += 1;
            let service_for_task = Arc::clone(&service);
            tauri::async_runtime::spawn_blocking(move || {
                let identity = job.req.app_identity.clone();
                let size = job.req.size;
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    let first = extract_and_cache(&job.req);
                    if first.is_ok() || job.req.executable_path.is_some() {
                        return first;
                    }
                    let hint = service_for_task
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .path_hints
                        .get(&identity)
                        .cloned();
                    let Some(hint) = hint else {
                        return first;
                    };
                    let mut retry = job.req.clone();
                    retry.executable_path = Some(hint.to_string_lossy().to_string());
                    extract_and_cache(&retry)
                }));
                let event = match outcome {
                    Ok(Ok(result)) => IconUpdateEvent {
                        app_identity: identity.clone(),
                        status: "resolved".into(),
                        cache_path: Some(result.png_path.to_string_lossy().to_string()),
                        icon_source: result.source.as_str().into(),
                        width: result.width,
                        height: result.height,
                        error_code: None,
                    },
                    Ok(Err(err)) => {
                        let code = err.to_string();
                        IconUpdateEvent {
                            app_identity: identity.clone(),
                            status: "failed".into(),
                            cache_path: None,
                            icon_source: IconSource::Fallback.as_str().into(),
                            width: size,
                            height: size,
                            error_code: Some(code.clone()),
                        }
                    }
                    Err(_) => IconUpdateEvent {
                        app_identity: identity.clone(),
                        status: "failed".into(),
                        cache_path: None,
                        icon_source: IconSource::Fallback.as_str().into(),
                        width: size,
                        height: size,
                        error_code: Some("icon_worker_panicked".into()),
                    },
                };

                {
                    let mut g = service_for_task
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    g.inflight.remove(&identity);
                    g.active = g.active.saturating_sub(1);
                    if event.status == "failed" {
                        if let Some(code) = event.error_code.clone() {
                            g.failures.insert(
                                identity.clone(),
                                FailureRecord {
                                    until: Instant::now() + FAILURE_COOLDOWN,
                                    code,
                                },
                            );
                            if g.failures.len() > MAX_FAILURES {
                                if let Some(oldest) = g.failures.keys().next().cloned() {
                                    g.failures.remove(&oldest);
                                }
                            }
                        }
                    } else {
                        g.failures.remove(&identity);
                    }
                    Self::pump_locked(&mut g, Arc::clone(&service_for_task));
                }

                let _ = job.app.emit("app-icon-updated", event);
            });
        }
    }
}

impl Default for IconService {
    fn default() -> Self {
        Self::new()
    }
}
