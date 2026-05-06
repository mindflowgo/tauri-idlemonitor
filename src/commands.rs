use tauri::{command, AppHandle, Runtime, State};

use crate::error::Result;
use crate::PowerMonitorState;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartOptions {
    pub idle_threshold_secs: Option<u64>,
}

#[command]
pub async fn start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PowerMonitorState>,
    options: Option<StartOptions>,
) -> Result<()> {
    let mut inner = state.0.lock().map_err(|e| crate::error::Error::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        e.to_string(),
    )))?;

    if inner.is_running() {
        return Err(crate::error::Error::AlreadyRunning);
    }

    let threshold = options.and_then(|o| o.idle_threshold_secs).unwrap_or(300);
    inner.set_threshold(threshold);
    inner.start_lock_listener(&app)?;
    inner.start_idle_monitor(&app);

    Ok(())
}

#[command]
pub async fn stop(state: State<'_, PowerMonitorState>) -> Result<()> {
    let mut inner = state.0.lock().map_err(|e| crate::error::Error::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        e.to_string(),
    )))?;

    inner.stop();
    Ok(())
}

#[command]
pub async fn get_idle_time() -> Result<serde_json::Value> {
    let secs = tokio::task::spawn_blocking(|| crate::platform::get_idle_seconds())
        .await
        .map_err(|e| crate::error::Error::Idle(e.to_string()))?
        .map_err(|e| crate::error::Error::Idle(e))?;

    Ok(serde_json::json!({ "seconds": secs }))
}
