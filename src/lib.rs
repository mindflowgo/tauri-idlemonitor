#![cfg(not(any(target_os = "android", target_os = "ios")))]

mod commands;
mod error;
mod idle;
mod platform;

use std::sync::{Arc, Mutex};

use tauri::{plugin::Builder as PluginBuilder, plugin::TauriPlugin, AppHandle, Manager, Runtime};

pub use error::{Error, IdlePayload, LockPayload, ResumePayload, SuspendPayload};

struct InnerState {
    idle_monitor: idle::IdleMonitor,
    lock_listener: Option<platform::LockListener>,
}

impl InnerState {
    fn new() -> Self {
        Self {
            idle_monitor: idle::IdleMonitor::new(300),
            lock_listener: None,
        }
    }

    fn set_threshold(&mut self, secs: u64) {
        self.idle_monitor = idle::IdleMonitor::new(secs);
    }

    fn start_lock_listener<R: Runtime>(&mut self, app: &AppHandle<R>) -> error::Result<()> {
        let listener = platform::start_lock_listener(app)
            .map_err(|e| error::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        self.lock_listener = Some(listener);
        Ok(())
    }

    fn start_idle_monitor<R: Runtime>(&mut self, app: &AppHandle<R>) {
        self.idle_monitor.start(app);
    }

    fn stop(&mut self) {
        self.idle_monitor.stop();
        if let Some(listener) = self.lock_listener.take() {
            (listener.stop)();
        }
    }

    fn is_running(&self) -> bool {
        self.idle_monitor.is_running()
    }
}

#[derive(Default)]
pub struct Builder {
    idle_threshold_secs: u64,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            idle_threshold_secs: 300,
        }
    }

    pub fn idle_threshold_secs(mut self, secs: u64) -> Self {
        self.idle_threshold_secs = secs;
        self
    }

    pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
        let threshold = self.idle_threshold_secs;
        PluginBuilder::new("idlemonitor")
            .invoke_handler(tauri::generate_handler![
                commands::start,
                commands::stop,
                commands::get_idle_time,
            ])
            .setup(move |app, _api| {
                let mut inner = InnerState::new();
                inner.set_threshold(threshold);
                app.manage(PowerMonitorState(Arc::new(Mutex::new(inner))));
                Ok(())
            })
            .build()
    }
}

pub struct PowerMonitorState(Arc<Mutex<InnerState>>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new().build()
}
