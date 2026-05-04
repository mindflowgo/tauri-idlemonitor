use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Runtime};

use crate::error::IdlePayload;
use crate::platform;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

pub struct IdleMonitor {
    running: Arc<AtomicBool>,
    is_idle: Arc<AtomicBool>,
    threshold_secs: u64,
}

impl IdleMonitor {
    pub fn new(threshold_secs: u64) -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            is_idle: Arc::new(AtomicBool::new(false)),
            threshold_secs,
        }
    }

    pub fn start<R: Runtime>(&mut self, app: &AppHandle<R>) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);
        self.is_idle.store(false, Ordering::Relaxed);

        let running = self.running.clone();
        let is_idle = self.is_idle.clone();
        let threshold = self.threshold_secs;
        let app = app.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create idle monitor runtime");

            rt.block_on(async move {
                let mut interval = tokio::time::interval(POLL_INTERVAL);

                while running.load(Ordering::Relaxed) {
                    interval.tick().await;

                    let secs = tokio::task::spawn_blocking(|| platform::get_idle_seconds())
                        .await
                        .unwrap_or(Err("spawn_blocking failed".into()))
                        .unwrap_or(0);

                    let was_idle = is_idle.load(Ordering::Relaxed);
                    let now_idle = secs >= threshold;

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
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}
