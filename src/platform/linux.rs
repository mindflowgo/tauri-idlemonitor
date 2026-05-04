use tauri::{AppHandle, Emitter, Runtime};

use crate::platform::types::LockListener;

pub fn start_lock_listener<R: Runtime>(app: &AppHandle<R>) -> std::result::Result<LockListener, String> {
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();

    let app_clone = app.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for linux lock listener");

        rt.block_on(listen_dbus(&app_clone, running_clone));
    });

    Ok(LockListener {
        stop: Box::new(move || {
            running.store(false, std::sync::atomic::Ordering::Relaxed);
        }),
    })
}

async fn listen_dbus<R: Runtime>(
    app: &AppHandle<R>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let conn = match zbus::Connection::session().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[idlemonitor] failed to connect to DBus session bus: {e}");
            return;
        }
    };

    let rule_ss = "type='signal',interface='org.freedesktop.ScreenSaver',member='ActiveChanged'";
    let rule_login = "type='signal',interface='org.freedesktop.login1.Manager',member='PrepareForSleep'";

    let _ = conn.add_match(rule_ss).await;
    let _ = conn.add_match(rule_login).await;

    let mut stream = zbus::MessageStream::from(&conn);

    loop {
        if !running.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        use futures_util::StreamExt;
        let msg = match tokio::time::timeout(
            std::time::Duration::from_secs(1),
            stream.next(),
        ).await {
            Ok(Some(Ok(m))) => m,
            _ => continue,
        };

        let header = msg.header();
        let iface = header.interface().map(|s| s.as_str());
        let member = header.member().map(|s| s.as_str());

        match (iface, member) {
            (Some("org.freedesktop.ScreenSaver"), Some("ActiveChanged")) => {
                if let Ok(active) = msg.body::<zbus::zvariant::Value<'_>>()
                    .and_then(|v| {
                        let b: bool = v.try_into()?;
                        Ok(b)
                    })
                {
                    let _ = app.emit("system:lock", crate::error::LockPayload { locked: active });
                } else if let Ok(active) = msg.body::<bool>() {
                    let _ = app.emit("system:lock", crate::error::LockPayload { locked: active });
                }
            }
            (Some("org.freedesktop.login1.Manager"), Some("PrepareForSleep")) => {
                if let Ok(sleeping) = msg.body::<bool>() {
                    if sleeping {
                        let _ = app.emit("system:suspend", crate::error::SuspendPayload {});
                    } else {
                        let _ = app.emit("system:resume", crate::error::ResumePayload {});
                    }
                }
            }
            _ => {}
        }
    }
}
