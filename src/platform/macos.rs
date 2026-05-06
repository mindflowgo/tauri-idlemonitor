use tauri::{AppHandle, Emitter, Runtime};

use crate::platform::types::LockListener;

pub fn start_lock_listener<R: Runtime>(app: &AppHandle<R>) -> std::result::Result<LockListener, String> {
    let app_clone = app.clone();
    
    // Register on main thread so the main runloop can process the NSDistributedNotificationCenter notifications
    let _ = app.run_on_main_thread(move || {
        unsafe { listen_nsworkspace(&app_clone) };
    });

    Ok(LockListener {
        stop: Box::new(|| {
            // Note: properly removing observers would require storing the id returned from addObserverForName
            // but for global plugin we just leave them active or they die with the app.
        }),
    })
}

unsafe fn listen_nsworkspace<R: Runtime>(app: &AppHandle<R>) {
    use objc2_foundation::{NSDistributedNotificationCenter, NSNotification, NSString};
    use std::ptr::NonNull;

    let center = NSDistributedNotificationCenter::defaultCenter();

    struct Handler {
        name: String,
        callback: Box<dyn Fn() + Send + Sync>,
    }

    let handlers: std::sync::Arc<Vec<Handler>> = std::sync::Arc::new(vec![
        Handler {
            name: "com.apple.screenIsLocked".into(),
            callback: {
                let app = app.clone();
                Box::new(move || {
                    let _ = app.emit("system:lock", crate::error::LockPayload { locked: true });
                })
            },
        },
        Handler {
            name: "com.apple.screenIsUnlocked".into(),
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
        
        let center_to_use = if handlers[i].name.contains("NSWorkspace") {
            use objc2_app_kit::NSWorkspace;
            NSWorkspace::sharedWorkspace().notificationCenter()
        } else {
            objc2::rc::Retained::into_super(center.clone())
        };

        unsafe {
            center_to_use.addObserverForName_object_queue_usingBlock(
                Some(&ns_name),
                None,
                None,
                &block,
            );
        }
        std::mem::forget(block);
    }
}
