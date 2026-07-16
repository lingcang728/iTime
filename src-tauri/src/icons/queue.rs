use super::extract::{extract_and_cache, try_cache_hit, ExtractRequest, IconSource};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const MAX_INFLIGHT: usize = 3;
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

pub struct IconService {
    inner: Arc<Mutex<IconServiceInner>>,
}

struct IconServiceInner {
    inflight: HashSet<String>,
    queued: Vec<PendingJob>,
    failures: HashMap<String, FailureRecord>,
    active: usize,
}

struct PendingJob {
    app: AppHandle,
    req: ExtractRequest,
}

impl IconService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(IconServiceInner {
                inflight: HashSet::new(),
                queued: Vec::new(),
                failures: HashMap::new(),
                active: 0,
            })),
        }
    }

    /// Cache-only fast path, otherwise enqueue background extraction (never blocks on Shell).
    pub fn try_get_cached_or_enqueue(
        &self,
        app: AppHandle,
        req: ExtractRequest,
    ) -> IconUpdateEvent {
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
            let mut guard = self.inner.lock().expect("icon service poisoned");

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

            guard.inflight.insert(identity.clone());
            guard.queued.push(PendingJob { app, req });
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
            let Some(job) = guard.queued.pop() else {
                break;
            };
            guard.active += 1;
            let service_for_task = Arc::clone(&service);
            tauri::async_runtime::spawn_blocking(move || {
                let identity = job.req.app_identity.clone();
                let size = job.req.size;
                let outcome = extract_and_cache(&job.req);
                let event = match outcome {
                    Ok(result) => IconUpdateEvent {
                        app_identity: identity.clone(),
                        status: "resolved".into(),
                        cache_path: Some(result.png_path.to_string_lossy().to_string()),
                        icon_source: result.source.as_str().into(),
                        width: result.width,
                        height: result.height,
                        error_code: None,
                    },
                    Err(err) => {
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
                };

                {
                    let mut g = service_for_task.lock().expect("icon service poisoned");
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
