use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Runtime};
use windows_sys::Win32::{
    Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM},
    System::RemoteDesktop::{
        WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        PostMessageW, RegisterClassW, UnregisterClassW, WM_WTSSESSION_CHANGE, WNDCLASSW,
        CW_USEDEFAULT,
    },
};

use crate::platform::types::LockListener;

// These constants are not exported by windows-sys 0.59
const WTS_SESSION_LOCK: u32 = 0x7;
const WTS_SESSION_UNLOCK: u32 = 0x8;

// The power-broadcast constants live in the Win32_System_Power feature we do
// not enable, so they are defined here to keep the feature set minimal.
const WM_POWERBROADCAST: u32 = 0x218;
const PBT_APMSUSPEND: u32 = 0x4;
const PBT_APMRESUMEAUTOMATIC: u32 = 0x12;

// WM_APP + 1, posted by stop() to break the blocking GetMessageW loop
const WM_APP_SHUTDOWN: u32 = 0x8001;

pub fn start_lock_listener<R: Runtime>(app: &AppHandle<R>) -> std::result::Result<LockListener, String> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // HWND is a raw pointer (not Send/Sync), so it is shared with stop() as an isize.
    let hwnd_slot: Arc<Mutex<Option<isize>>> = Arc::new(Mutex::new(None));
    let hwnd_slot_clone = hwnd_slot.clone();

    let app_clone = app.clone();
    std::thread::spawn(move || unsafe {
        listen_wts(&app_clone, running_clone, hwnd_slot_clone);
    });

    Ok(LockListener {
        stop: Box::new(move || {
            running.store(false, Ordering::Relaxed);
            if let Ok(slot) = hwnd_slot.lock() {
                if let Some(hwnd) = *slot {
                    unsafe { PostMessageW(hwnd as HWND, WM_APP_SHUTDOWN, 0, 0) };
                }
            }
        }),
    })
}

unsafe fn listen_wts<R: Runtime>(
    app: &AppHandle<R>,
    running: Arc<AtomicBool>,
    hwnd_slot: Arc<Mutex<Option<isize>>>,
) {
    let class_name: Vec<u16> = "TauriIdleMonitor\0".encode_utf16().collect();

    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(def_window_proc),
        lpszClassName: class_name.as_ptr(),
        ..unsafe { std::mem::zeroed() }
    };

    unsafe { RegisterClassW(&wnd_class) };

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::mem::zeroed(),
            std::ptr::null_mut(),
        )
    };

    if hwnd.is_null() {
        eprintln!(
            "[idlemonitor] failed to create Windows listener window (GetLastError={})",
            unsafe { GetLastError() }
        );
        return;
    }

    if unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) } == 0 {
        eprintln!(
            "[idlemonitor] WTSRegisterSessionNotification failed (GetLastError={}); lock/unlock events will not fire",
            unsafe { GetLastError() }
        );
    }

    if let Ok(mut slot) = hwnd_slot.lock() {
        *slot = Some(hwnd as isize);
    }

    // stop() may have run before the window existed
    if !running.load(Ordering::Relaxed) {
        cleanup(hwnd, &class_name, &hwnd_slot);
        return;
    }

    let mut msg = std::mem::zeroed();
    loop {
        // GetMessageW blocks until a message arrives; 0 means WM_QUIT, -1 means error
        let ret = unsafe { GetMessageW(&mut msg, hwnd, 0, 0) };
        if ret <= 0 {
            if ret == -1 {
                eprintln!(
                    "[idlemonitor] GetMessageW failed (GetLastError={})",
                    unsafe { GetLastError() }
                );
            }
            break;
        }

        match msg.message {
            WM_WTSSESSION_CHANGE => match msg.wParam as u32 {
                WTS_SESSION_LOCK => {
                    let _ = app.emit("system:lock", crate::error::LockPayload { locked: true });
                }
                WTS_SESSION_UNLOCK => {
                    let _ = app.emit("system:lock", crate::error::LockPayload { locked: false });
                }
                _ => {}
            },
            WM_POWERBROADCAST => match msg.wParam as u32 {
                PBT_APMSUSPEND => {
                    let _ = app.emit("system:suspend", crate::error::SuspendPayload {});
                }
                PBT_APMRESUMEAUTOMATIC => {
                    let _ = app.emit("system:resume", crate::error::ResumePayload {});
                }
                _ => {}
            },
            WM_APP_SHUTDOWN => break,
            _ => {}
        }

        unsafe { DispatchMessageW(&msg) };
    }

    cleanup(hwnd, &class_name, &hwnd_slot);
}

unsafe fn cleanup(hwnd: HWND, class_name: &[u16], hwnd_slot: &Mutex<Option<isize>>) {
    if let Ok(mut slot) = hwnd_slot.lock() {
        *slot = None;
    }
    unsafe {
        WTSUnRegisterSessionNotification(hwnd);
        DestroyWindow(hwnd);
        UnregisterClassW(class_name.as_ptr(), std::ptr::null_mut());
    }
}

unsafe extern "system" fn def_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
