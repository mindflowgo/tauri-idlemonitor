use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Runtime};

use crate::error::IdlePayload;
use crate::platform;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

pub struct IdleMonitor {
    running: Arc<AtomicBool>,
    is_idle: Arc<AtomicBool>,
    threshold_secs: Arc<AtomicU64>,
}

impl IdleMonitor {
    pub fn new(threshold_secs: u64) -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            is_idle: Arc::new(AtomicBool::new(false)),
            threshold_secs: Arc::new(AtomicU64::new(threshold_secs)),
        }
    }

    pub fn set_threshold(&self, secs: u64) {
        self.threshold_secs.store(secs, Ordering::Relaxed);
    }

    pub fn start<R: Runtime>(&mut self, app: &AppHandle<R>) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);
        self.is_idle.store(false, Ordering::Relaxed);

        let running = self.running.clone();
        let is_idle = self.is_idle.clone();
        let threshold = self.threshold_secs.clone();
        let app = app.clone();

        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(POLL_INTERVAL);
            let mut error_logged = false;

            while running.load(Ordering::Relaxed) {
                interval.tick().await;

                let idle_result = tokio::task::spawn_blocking(platform::get_idle_seconds)
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()));

                let secs = match idle_result {
                    Ok(secs) => {
                        error_logged = false;
                        secs
                    }
                    Err(e) => {
                        // A persistent failure here would otherwise look like "never idle"
                        if !error_logged {
                            eprintln!("[idlemonitor] failed to read idle time: {e}");
                            error_logged = true;
                        }
                        0
                    }
                };

                let was_idle = is_idle.load(Ordering::Relaxed);
                let now_idle = secs >= threshold.load(Ordering::Relaxed);

                if !was_idle && now_idle {
                    is_idle.store(true, Ordering::Relaxed);
                    let _ = app.emit(
                        "system:idle",
                        IdlePayload {
                            idle: true,
                            seconds: Some(secs),
                        },
                    );
                } else if was_idle && !now_idle {
                    is_idle.store(false, Ordering::Relaxed);
                    let _ = app.emit(
                        "system:idle",
                        IdlePayload {
                            idle: false,
                            seconds: None,
                        },
                    );
                }
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}
