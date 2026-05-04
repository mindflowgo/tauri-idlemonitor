use tauri::{AppHandle, Emitter, Runtime};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::RemoteDesktop::{
        WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
        RegisterClassW, WM_WTSSESSION_CHANGE, WNDCLASSW, CW_USEDEFAULT,
    },
};

use crate::platform::types::LockListener;

// These constants are not exported by windows-sys 0.59
const WTS_SESSION_LOCK: u32 = 0x7;
const WTS_SESSION_UNLOCK: u32 = 0x8;

pub fn start_lock_listener<R: Runtime>(app: &AppHandle<R>) -> std::result::Result<LockListener, String> {
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();

    let app_clone = app.clone();
    std::thread::spawn(move || unsafe {
        listen_wts(&app_clone, running_clone);
    });

    Ok(LockListener {
        stop: Box::new(move || {
            running.store(false, std::sync::atomic::Ordering::Relaxed);
        }),
    })
}

unsafe fn listen_wts<R: Runtime>(
    app: &AppHandle<R>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let class_name: Vec<u16> = "TauriPowerMonitor\0".encode_utf16().collect();

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
        return;
    }

    unsafe {
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
    }

    let mut msg = std::mem::zeroed();
    while running.load(std::sync::atomic::Ordering::Relaxed) {
        let ret = unsafe { GetMessageW(&mut msg, hwnd, 0, 0) };
        if ret == 0 {
            break;
        }

        if msg.message == WM_WTSSESSION_CHANGE {
            match msg.wParam as u32 {
                WTS_SESSION_LOCK => {
                    let _ = app.emit("power:lock", crate::error::LockPayload { locked: true });
                }
                WTS_SESSION_UNLOCK => {
                    let _ = app.emit("power:lock", crate::error::LockPayload { locked: false });
                }
                _ => {}
            }
        }

        unsafe { DispatchMessageW(&msg) };
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
