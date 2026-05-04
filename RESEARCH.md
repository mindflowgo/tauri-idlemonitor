# Research
Final coding by Z.AI v5.1 with MCP assistance from: 
[brAInstorm](https://github.com/mindflowgo/brAInstorm)

Ideas, and initial research, and technical guidance by Filipe Laborde (fil@rezox.com)


## GOAL
The goal is to make clean Tauri v2 plugin that will make available to the app the following features:
- notify when lockscreen comes on
- notify when lockscreen is turned off (ie login again)
- notify of idle (and how long idle)
    - can set the time before considers idle
    - uses whatever is best (mouse movement, other events?)
- notify when breaking idle
- works on Mac, Windows, and ideally linux

Use guidance for Tauri plugin development: 
https://v2.tauri.app/develop/plugins/

## PRIOR ART EXAMINATION
We will start by researching what projects exist and how they achieve it:
 
1) Idletime tracker for node:
https://github.com/anaisbetts/node-system-idle-time

2) In Electron they have a plugin called powerMonitor (that has listeners for lockscreen, idletime). Also can do with screen.getCursorScreenPoint() for mouse movement.

3) ** This node oen looks recently maintained (at least gives cross-platform idle detection):
https://github.com/MomoRazor/node-desktop-idle-v2


# Gemini Research

Below is the breakdown of the underlying Electron source code and the equivalent system APIs you would need to use in Rust for a Tauri plugin.

### 1. Where to find the Electron Source Code
Electron's power monitoring logic is located in the `shell/browser/api/` and `shell/browser/` directories of the [electron/electron](https://github.com/electron/electron) repository. Specifically, look at the **PowerMonitor** class implementation:

*   **Header File:** [`shell/browser/api/electron_api_power_monitor.h`](https://github.com/electron/electron/blob/main/shell/browser/api/electron_api_power_monitor.h)
*   **Linux (D-Bus):** [`shell/browser/power_monitor_linux.cc`](https://github.com/electron/electron/blob/main/shell/browser/power_monitor_linux.cc)
*   **macOS (IOPublishPort):** [`shell/browser/power_monitor_mac.mm`](https://github.com/electron/electron/blob/main/shell/browser/power_monitor_mac.mm)
*   **Windows (WM_POWERBROADCAST):** [`shell/browser/power_monitor_win.cc`](https://github.com/electron/electron/blob/main/shell/browser/power_monitor_win.cc)

---

### 2. OS-Specific Logic (For your Tauri Plugin)
If you are building this in Rust for Tauri, you will likely need to use the following system calls via crates like `windows-sys`, `cocoa`, or `zbus`.

#### **Windows**
Electron listens for the `WM_POWERBROADCAST` message in a hidden window's message loop.
*   **Suspend:** Look for `PBT_APMSUSPEND`.
*   **Resume:** Look for `PBT_APMRESUMESUSPEND`.
*   **Rust Tip:** Use the `windows` crate to create a hidden window and poll for these messages.

#### **macOS**
Electron uses the `IOPMrootDomain` and `IONotificationPortRef` to listen for system sleep/wake notifications.
*   **Suspend:** `kIOMessageSystemWillSleep`.
*   **Resume:** `kIOMessageSystemHasPoweredOn`.
*   **Rust Tip:** Use the `apple-sys` or `cocoa` crates to register for `io_publisher_t` notifications.

#### **Linux**
Electron generally relies on **DBus** to communicate with `org.freedesktop.login1`.
*   **Signals:** It listens for `PrepareForSleep` (true = suspending, false = resuming) on the `/org/freedesktop/login1` object.
*   **Rust Tip:** Use the `zbus` crate to subscribe to these specific DBus signals.

---

# Key Finding: `system-idle-time` Limitation

**`system-idle-time` only supports X11 (XScreenSaver) on Linux — NO Wayland support.**
The `node-desktop-idle-v2` project confirmed this by switching from XScreenSaver to libinput for Linux.

**Strategy**: Use `user-idle2` crate for idle detection (already implements the platform fallback chain we need). Implement lock/unlock detection ourselves with platform-specific modules. Current scope covers macOS, Windows, and Linux X11 + GNOME Wayland. KDE/Sway/Hyprland Wayland is documented as a future upgrade.

## Current Scope (v1)
- macOS: `user-idle2` → IOKit/IOHIDSystem (idle) + NSWorkspace notifications (lock)
- Windows: `user-idle2` → GetLastInputInfo (idle) + WTS session notifications (lock)
- Linux: `user-idle2` → GNOME Mutter DBus → XScreenSaver fallback (idle) + zbus login1/ScreenSaver (lock)

## Future Upgrade (v2) — Full Linux Wayland
For non-GNOME Wayland compositors (KDE, Sway, Hyprland):
- **KDE**: `org.freedesktop.ScreenSaver.GetActiveTime()` via zbus
- **Minimal compositors** (Sway, River, Niri): `ext-idle-notify-v1` Wayland protocol or may not work
- **Lock/unlock**: `org.freedesktop.login1` `SessionLock`/`SessionUnlock` signals (already works on both X11 and Wayland)

---

# Architecture Plan

## Crates & Dependencies

| Feature | Rust Crate | Notes |
|---------|-----------|-------|
| Idle time (all platforms) | `user-idle2` v0.3+ | Cross-platform: IOKit (macOS), GetLastInputInfo (Win), Mutter DBus → XScreenSaver (Linux). Wrap in `spawn_blocking` for async. |
| macOS lock | `core-foundation` + `objc` | NSWorkspace notifications |
| Windows lock | `windows` crate | `WTSRegisterSessionNotificationEx` → `WM_WTSSESSION_CHANGE` |
| Linux lock | `zbus` | `org.freedesktop.login1` `PrepareForSleep` + `org.freedesktop.ScreenSaver.ActiveChanged` |
| Tauri events | `tauri` built-in | `app_handle.emit_all("system:lock", payload)` |

## Plugin Structure

```
tauri-plugin-idlemonitor/
├── src/
│   ├── lib.rs              # Plugin builder, state management, command registration
│   ├── commands.rs          # #[command] fns: start(), stop(), get_idle_time()
│   ├── idle.rs              # Idle polling loop + threshold state machine
│   ├── error.rs             # Error types
│   └── platform/
│       ├── mod.rs           # Platform dispatch via cfg(target_os)
│       ├── macos.rs         # CGEventSource idle + NSWorkspace lock notifications
│       ├── windows.rs       # GetLastInputInfo idle + WTS session lock listener
│       ├── linux.rs         # X11 XScreenSaver idle + DBus lock listeners
│       └── types.rs         # Shared types: IdleState, LockState, PowerEvent
├── guest-js/
│   └── index.ts             # Typed JS API: start(), stop(), get_idle_time(), listen()
├── permissions/
│   ├── default.toml         # allow-start, allow-stop, allow-get-idle-time
│   └── full.toml            # All permissions
├── build.rs                 # Permission autogeneration
├── Cargo.toml
└── package.json             # NPM package for guest-js bindings
```

## Events Emitted (Rust → JS)

| Event | Payload | Trigger |
|-------|---------|---------|
| `system:lock` | `{ locked: true }` | Screen locked |
| `system:lock` | `{ locked: false }` | Screen unlocked |
| `system:idle` | `{ idle: true, seconds: 300 }` | Idle threshold crossed |
| `system:idle` | `{ idle: false }` | User activity after idle |
| `system:suspend` | `{ }` | System sleep (bonus) |
| `system:resume` | `{ }` | System wake (bonus) |

## Commands (JS → Rust)

| Command | Params | Returns | Description |
|---------|--------|---------|-------------|
| `plugin:idlemonitor|start` | `{ idle_threshold_secs?: u64 }` | `void` | Start all listeners + idle polling |
| `plugin:idlemonitor|stop` | `{ }` | `void` | Stop all listeners and polling |
| `plugin:idlemonitor|get_idle_time` | `{ }` | `{ seconds: u64 }` | One-shot idle time query |

## Platform-Specific Implementation Detail

### Idle Time — All Platforms (via `user-idle2`)
```rust
use user_idle::UserIdle;
use std::time::Duration;

// Works on macOS (IOKit), Windows (GetLastInputInfo), Linux (Mutter DBus → XScreenSaver)
// Wrap in tokio::task::spawn_blocking to avoid blocking async runtime
async fn get_idle_time_secs() -> u64 {
    tokio::task::spawn_blocking(|| {
        UserIdle::get_time()
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }).await.unwrap_or(0)
}
```

**`user-idle2` fallback chain on Linux:**
1. Try `org.gnome.Mutter.IdleMonitor.GetIdletime` via DBus (GNOME, works on X11 + Wayland)
2. Fall back to XScreenSaver extension via `x11` crate (X11 only)
3. Fall back to `org.freedesktop.ScreenSaver.GetSessionIdleTime` (often unimplemented)

### macOS — Lock/Unlock
Use NSWorkspace distributed notifications via `objc` crate:
- `NSWorkspaceSessionDidResignActiveNotification` → locked
- `NSWorkspaceSessionDidBecomeActiveNotification` → unlocked
- `NSWorkspaceScreensDidSleepNotification` → display sleep (bonus)
- `NSWorkspaceScreensDidWakeNotification` → display wake (bonus)

Register via `[NSWorkspace sharedWorkspace].notificationCenter`.

### Windows — Lock/Unlock
Create hidden window, call `WTSRegisterSessionNotificationEx`, listen for `WM_WTSSESSION_CHANGE`:
- `WTS_SESSION_LOCK` = screen locked
- `WTS_SESSION_UNLOCK` = screen unlocked

### Linux — Lock/Unlock (DBus via zbus)
```rust
// org.freedesktop.login1 PrepareForSleep signal → sleep/wake
// org.freedesktop.ScreenSaver ActiveChanged(bool) → screen lock state
```

### Linux Wayland — Idle Time (FUTURE v2 upgrade)
```rust
use zbus::{Connection, proxy};

// GNOME (returns milliseconds)
#[proxy(interface = "org.gnome.Mutter.IdleMonitor", default_service = "org.gnome.Shell",
        default_path = "/org/gnome/Mutter/IdleMonitor/Core")]
trait IdleMonitor { fn get_idletime(&self) -> zbus::Result<u64>; }

// KDE/Generic (returns seconds)
#[proxy(interface = "org.freedesktop.ScreenSaver", default_service = "org.freedesktop.ScreenSaver",
        default_path = "/org/freedesktop/ScreenSaver")]
trait ScreenSaver { fn get_active_time(&self) -> zbus::Result<u32>; }

fn get_idle_time_wayland() -> u64 {
    // Try Mutter first (GNOME), fall back to ScreenSaver (KDE/generic)
    // Gracefully handle compositors that don't implement either (Sway, River, etc.)
}
```

### Linux Wayland — Lock/Unlock (FUTURE v2 upgrade)
```rust
// org.freedesktop.login1 SessionLock/SessionUnlock signals
// Already works on both X11 and Wayland via zbus
```

## Idle Polling Strategy

- Tokio interval every 3 seconds
- Call platform-specific `get_idle_time()`
- Compare against configurable threshold (default 300s)
- Only emit `system:idle` on **state transitions** (not-idle→idle, idle→not-idle)
- Lock/unlock listeners are **event-driven** (zero polling)

## guest-js API

```typescript
import { start, stop, getIdleTime } from 'tauri-plugin-idlemonitor-api';
import { listen } from '@tauri-apps/api/event';

// Start monitoring (optional idle threshold, default 300s)
await start({ idleThresholdSecs: 300 });

// Listen for events
await listen('system:lock', (e) => {
  console.log(e.payload.locked ? 'Screen locked' : 'Screen unlocked');
});
await listen('system:idle', (e) => {
  console.log(e.payload.idle ? `Idle ${e.payload.seconds}s` : 'Active');
});
await listen('system:suspend', () => console.log('System sleeping'));
await listen('system:resume', () => console.log('System woke'));

// One-shot query
const { seconds } = await getIdleTime();

// Stop
await stop();
```

## Implementation Order

1. **Plugin scaffold** — Cargo.toml, lib.rs, build.rs, permissions, guest-js
2. **Shared types** — `error.rs`, `platform/types.rs` (IdleState, LockState, PowerEvent)
3. **Idle time** — `user-idle2` integration with `spawn_blocking` wrapper
4. **macOS lock** — NSWorkspace notification listener
5. **Windows lock** — WTSRegisterSessionNotificationEx + hidden window
6. **Linux lock** — zbus login1 + ScreenSaver ActiveChanged signals
7. **Idle polling loop** — tokio interval + threshold state machine
8. **Event emission** — Tauri emit_all for all power events
9. **Commands** — start(), stop(), get_idle_time()
10. **guest-js** — TypeScript API wrapper
11. **Testing** — macOS first (primary dev platform), then Windows, then Linux

## Future Work (v2)

| Feature | Approach | Notes |
|---------|----------|-------|
| Linux Wayland idle | `zbus` → Mutter.IdleMonitor + ScreenSaver fallback | Runtime `WAYLAND_DISPLAY` check, trait-based platform dispatch |
| Linux Wayland lock | `zbus` → login1 SessionLock/SessionUnlock | Same crate, different signals |
| Minimal compositors (Sway, Hyprland) | May need compositor-specific protocols or idle-inhibit | No standardized DBus idle API exists |
| System power status | AC vs battery, battery percentage | Platform-specific power APIs |