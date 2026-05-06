# tauri-plugin-idlemonitor

Monitor screen lock/unlock, system idle time, and suspend/resume events in Tauri v2 applications. Inspired by Electron's `powerMonitor` API.

## Demo-App Previewing Plugin

![idlemonitor](https://raw.githubusercontent.com/mindflowgo/tauri-idlemonitor/main/preview.jpg)

## Platform Support

| Platform | Lock/Unlock | Idle Time | Suspend/Resume | Notes |
| -------- | ----------- | --------- | -------------- | ----- |
| macOS    | ✓           | ✓         | ✓              | NSWorkspace notifications + IOKit |
| Windows  | ✓           | ✓         | —              | WTS session notifications + GetLastInputInfo |
| Linux    | ✓           | ✓         | ✓              | DBus (ScreenSaver + login1) + XScreenSaver/GNOME Mutter |
| Android  | x           | x         | x              | Not supported |
| iOS      | x           | x         | x              | Not supported |

## Crate v0.2.2
[tauri-plugin-idlemonitor](https://crates.io/crates/tauri-plugin-idlemonitor)

## Install

### Rust

Add to your `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-idlemonitor = "0.2.2"
```

Or from a git repository:

```toml
[dependencies]
tauri-plugin-idlemonitor = { git = "https://github.com/mindflowgo/tauri-idlemonitor/" }
```

### JavaScript/TypeScript

Copy the `tauri-plugin-idlemonitor-api/` directory into your javascript/typescript project.

### Permissions

Add to your `src-tauri/capabilities/default.json`:

```json
{
  "permissions": [
    "idlemonitor:default"
  ]
}
```

## Usage

### 1. Register the Plugin (Rust)

`src-tauri/src/lib.rs`

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_idlemonitor::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Or with a custom idle threshold:

```rust
tauri::Builder::default()
    .plugin(
        tauri_plugin_idlemonitor::Builder::new()
            .idle_threshold_secs(600) // 10 minutes
            .build()
    )
```

### 2. Start Monitoring (Frontend)

```typescript
import { start, stop, getIdleTime, onLock, onIdle, onSuspend, onResume } from 'tauri-plugin-idlemonitor-api'

// Start monitoring with a 5-minute idle threshold (default: 300 seconds)
await start({ idleThresholdSecs: 300 })
```

### 3. Listen for Events

#### Screen Lock/Unlock

```typescript
import { onLock } from 'tauri-plugin-idlemonitor-api'

const unlisten = await onLock((payload) => {
  if (payload.locked) {
    console.log('Screen locked — pausing timers')
    pauseUserTimers()
  } else {
    console.log('Screen unlocked — resuming')
    resumeUserTimers()
  }
})

// Later: stop listening
unlisten()
```

#### Idle Detection

```typescript
import { onIdle } from 'tauri-plugin-idlemonitor-api'

const unlisten = await onIdle((payload) => {
  if (payload.idle) {
    console.log(`User went idle (${payload.seconds}s) — saving state`)
    autoSaveState()
  } else {
    console.log('User is back — refreshing data')
    refreshFromServer()
  }
})
```

#### Suspend/Resume

```typescript
import { onSuspend, onResume } from 'tauri-plugin-idlemonitor-api'

await onSuspend(() => {
  console.log('System going to sleep — saving all work')
  saveAllWork()
})

await onResume(() => {
  console.log('System woke up — reconnecting')
  reconnectWebSocket()
})
```

### 4. Query Idle Time Directly

```typescript
import { getIdleTime } from 'tauri-plugin-idlemonitor-api'

const { seconds } = await getIdleTime()
console.log(`User has been idle for ${seconds} seconds`)
```

### 5. Stop Monitoring

```typescript
import { stop } from 'tauri-plugin-idlemonitor-api'

await stop()
```

## Complete Example

Here's a full example showing a productivity timer that pauses when the screen locks or the user goes idle:

```typescript
import {
  start,
  stop,
  onLock,
  onIdle,
  onResume
} from 'tauri-plugin-idlemonitor-api'

let isTracking = true

async function setupIdleMonitoring() {
  // Start with 5-minute idle threshold
  await start({ idleThresholdSecs: 300 })

  // Pause timer when screen locks
  await onLock((payload) => {
    if (payload.locked) {
      isTracking = false
      saveTimerState()
      console.log('⏸ Paused: screen locked')
    } else {
      isTracking = true
      console.log('▶ Resumed: screen unlocked')
    }
  })

  // Handle idle state
  await onIdle((payload) => {
    if (payload.idle) {
      isTracking = false
      saveTimerState()
      console.log(`⏸ Paused: user idle for ${payload.seconds}s`)
    } else {
      isTracking = true
      console.log('▶ Resumed: user active again')
    }
  })

  // Reconnect on wake
  await onResume(() => {
    syncDataToServer()
    console.log('System woke up — syncing data')
  })
}

// Call on app startup
setupIdleMonitoring()
```

## API Reference

### `start(options?)`

Start all idle monitoring listeners and idle polling.

```typescript
function start(options?: { idleThresholdSecs?: number }): Promise<void>
```

- `idleThresholdSecs` — Seconds of inactivity before emitting an idle event. Default: `300` (5 minutes).

### `stop()`

Stop all monitoring and release system resources.

```typescript
function stop(): Promise<void>
```

### `getIdleTime()`

Query the current system idle time. Does not require monitoring to be started.

```typescript
function getIdleTime(): Promise<{ seconds: number }>
```

### `onLock(handler)`

Listen for screen lock/unlock events.

```typescript
function onLock(handler: (payload: { locked: boolean }) => void): Promise<() => void>
```

Returns an `unlisten` function.

### `onIdle(handler)`

Listen for idle state changes (crosses the threshold, or returns from idle).

```typescript
function onIdle(handler: (payload: { idle: boolean; seconds?: number }) => void): Promise<() => void>
```

- `idle: true` — `seconds` indicates how long the user has been idle
- `idle: false` — user just became active again

### `onSuspend(handler)`

Listen for system sleep events.

```typescript
function onSuspend(handler: () => void): Promise<() => void>
```

### `onResume(handler)`

Listen for system wake events.

```typescript
function onResume(handler: () => void): Promise<() => void>
```

## Events

| Event | Payload | Description |
|-------|---------|-------------|
| `system:lock` | `{ locked: boolean }` | Screen locked or unlocked |
| `system:idle` | `{ idle: boolean, seconds?: number }` | User idle threshold crossed or broken |
| `system:suspend` | `{ }` | System going to sleep |
| `system:resume` | `{ }` | System waking up |

You can also listen to raw events using `@tauri-apps/api/event`:

```typescript
import { listen } from '@tauri-apps/api/event'

await listen('system:lock', (event) => {
  console.log(event.payload) // { locked: true/false }
})
```

## Architecture

```
┌─────────────┐     emit()      ┌──────────────┐
│  Platform    │ ──────────────> │  Tauri Core  │
│  Listeners   │   (Rust)        │   Events     │
├─────────────┤                 └──────┬───────┘
│ macOS:      │                        │
│  NSWorkspc  │                 ┌──────┴───────┐
│ Windows:    │                 │  guest-js    │
│  WTS msgs   │                 │  TypeScript  │
│ Linux:      │                 │  API         │
│  DBus/zbus  │                 └──────────────┘
├─────────────┤
│ Idle Timer  │
│ (3s poll)   │
│ user-idle2  │
└─────────────┘
```

## Performance

- **Lock/unlock listeners** are event-driven — zero CPU usage when idle
- **Idle time polling** runs every 3 seconds via `tokio::time::interval`, using `spawn_blocking` to avoid blocking the async runtime
- **State-change-only emission** — events fire only on transitions (not-idle→idle, idle→not-idle), not on every poll

## Underlying System APIs

### macOS
- **Idle time**: IOKit `kIOHIDLastActivityTimeKey` via the `user-idle2` crate
- **Lock/Unlock**: `NSDistributedNotificationCenter` — listens for `NSWorkspaceSessionDidResignActiveNotification` (lock) and `NSWorkspaceSessionDidBecomeActiveNotification` (unlock)
- **Suspend/Resume**: `NSWorkspaceScreensDidSleepNotification` and `NSWorkspaceScreensDidWakeNotification`

### Windows
- **Idle time**: `GetLastInputInfo()` Win32 API — returns tick count of last input event
- **Lock/Unlock**: `WTSRegisterSessionNotificationEx` — receives `WM_WTSSESSION_CHANGE` messages with `WTS_SESSION_LOCK` / `WTS_SESSION_UNLOCK` via a hidden window

### Linux

Because the linux GUI ecosystem is so more more diverse, different techiques were used:

| Display Server | Idle Detection | Lock Detection |
| -------------- | -------------- | -------------- |
| X11            | ✓ XScreenSaver extension | ✓ DBus ScreenSaver + login1 |
| Wayland (GNOME)| ✓ Mutter IdleMonitor DBus | ✓ DBus ScreenSaver + login1 |
| Wayland (KDE)  | Partial | ✓ DBus ScreenSaver + login1 |
| Wayland (Sway/Hyprland) | Not yet | ✓ DBus login1 |

- **Idle time (X11)**: XScreenSaver extension `XScreenSaverQueryInfo` via the `x11` crate
- **Idle time (GNOME Wayland)**: `org.gnome.Mutter.IdleMonitor.GetIdletime` via DBus
- **Lock/Unlock**: `org.freedesktop.ScreenSaver.ActiveChanged` DBus signal
- **Suspend/Resume**: `org.freedesktop.login1.Manager.PrepareForSleep` DBus signal

## Future Work

| Feature | Status | Notes |
|---------|--------|-------|
| Linux Wayland idle (KDE, Sway) | Planned | Will use `org.freedesktop.ScreenSaver.GetActiveTime()` via zbus |
| Linux Wayland lock (all compositors) | Planned | `org.freedesktop.login1.SessionLock`/`SessionUnlock` signals |
| Windows suspend/resume | Planned | `WM_POWERBROADCAST` / `PBT_APMSUSPEND` / `PBT_APMRESUMESUSPEND` |
| Battery status | Planned | Platform-specific power APIs |

## License

MIT

## Contributions

Original concept researched and guided by Filipe Laborde (fil {at} rezox.com), modules, prefered method calls; however the heavy lifting was done by Z.Ai v5.1.
