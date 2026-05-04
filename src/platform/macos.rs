use tauri::{AppHandle, Emitter, Runtime};

use crate::platform::types::LockListener;

pub fn start_lock_listener<R: Runtime>(app: &AppHandle<R>) -> std::result::Result<LockListener, String> {
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();

    let app_clone = app.clone();
    std::thread::spawn(move || {
        unsafe { listen_nsworkspace(&app_clone, running_clone) };
    });

    Ok(LockListener {
        stop: Box::new(move || {
            running.store(false, std::sync::atomic::Ordering::Relaxed);
        }),
    })
}

unsafe fn listen_nsworkspace<R: Runtime>(
    app: &AppHandle<R>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use objc2_foundation::{NSDistributedNotificationCenter, NSNotification, NSString};
    use std::ptr::NonNull;

    let center = NSDistributedNotificationCenter::defaultCenter();

    struct Handler {
        name: String,
        callback: Box<dyn Fn() + Send + Sync>,
    }

    let handlers: std::sync::Arc<Vec<Handler>> = std::sync::Arc::new(vec![
        Handler {
            name: "NSWorkspaceSessionDidResignActiveNotification".into(),
            callback: {
                let app = app.clone();
                Box::new(move || {
                    let _ = app.emit("system:lock", crate::error::LockPayload { locked: true });
                })
            },
        },
        Handler {
            name: "NSWorkspaceSessionDidBecomeActiveNotification".into(),
            callback: {
                let app = app.clone();
                Box::new(move || {
                    let _ = app.emit("system:lock", crate::error::LockPayload { locked: false });
                })
            },
        },
        Handler {
            name: "NSWorkspaceScreensDidSleepNotification".into(),
            callback: {
                let app = app.clone();
                Box::new(move || {
                    let _ = app.emit("system:suspend", crate::error::SuspendPayload {});
                })
            },
        },
        Handler {
            name: "NSWorkspaceScreensDidWakeNotification".into(),
            callback: {
                let app = app.clone();
                Box::new(move || {
                    let _ = app.emit("system:resume", crate::error::ResumePayload {});
                })
            },
        },
    ]);

    for i in 0..handlers.len() {
        let handlers_clone = handlers.clone();
        let block = block2::StackBlock::new(move |_note: NonNull<NSNotification>| {
            if let Some(h) = handlers_clone.get(i) {
                (h.callback)();
            }
        }).copy();

        let ns_name = NSString::from_str(&handlers[i].name);
        unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(&ns_name),
                None,
                None,
                &block,
            );
        }
        std::mem::forget(block);
    }

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
