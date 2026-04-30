mod types;

pub use types::LockListener;

use tauri::{AppHandle, Runtime};

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

pub fn get_idle_seconds() -> std::result::Result<u64, String> {
    user_idle2::UserIdle::get_time()
        .map(|idle| idle.as_seconds())
        .map_err(|e| e.to_string())
}

pub fn start_lock_listener<R: Runtime>(app: &AppHandle<R>) -> std::result::Result<LockListener, String> {
    #[cfg(target_os = "macos")]
    {
        macos::start_lock_listener(app)
    }
    #[cfg(target_os = "windows")]
    {
        windows::start_lock_listener(app)
    }
    #[cfg(target_os = "linux")]
    {
        linux::start_lock_listener(app)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = app;
        Err("unsupported platform".into())
    }
}
