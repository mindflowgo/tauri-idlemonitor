// Complete example: Productivity timer that reacts to screen lock, idle, and system sleep.
//
// Usage:
//   1. Register the plugin in src-tauri/src/lib.rs:
//      tauri::Builder::default().plugin(tauri_plugin_powermonitor::init())
//
//   2. Import and call setupPowerMonitoring() on app startup.

import {
  start,
  stop,
  onLock,
  onIdle,
  onSuspend,
  onResume,
  getIdleTime,
} from 'tauri-plugin-idlemonitor-api'

let isTimerRunning = false
let timerSeconds = 0
let timerInterval: ReturnType<typeof setInterval> | null = null

function startTimer() {
  if (timerInterval) return
  isTimerRunning = true
  timerInterval = setInterval(() => {
    timerSeconds++
    updateTimerDisplay()
  }, 1000)
}

function pauseTimer() {
  isTimerRunning = false
  if (timerInterval) {
    clearInterval(timerInterval)
    timerInterval = null
  }
}

function updateTimerDisplay() {
  const mins = Math.floor(timerSeconds / 60)
  const secs = timerSeconds % 60
  console.log(`⏱ ${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`)
}

async function saveState() {
  console.log(`💾 Saving timer state: ${timerSeconds}s elapsed`)
  // In a real app, save to localStorage, IndexedDB, or backend
}

async function syncToServer() {
  console.log('🔄 Syncing data to server after wake...')
}

export async function setupPowerMonitoring() {
  // Start monitoring with 5-minute (300s) idle threshold
  await start({ idleThresholdSecs: 300 })
  startTimer()
  console.log('✅ Power monitoring started — timer running')

  // ── Screen Lock ──────────────────────────────────────────
  await onLock((payload) => {
    if (payload.locked) {
      pauseTimer()
      saveState()
      console.log('⏸ Screen LOCKED — timer paused, state saved')
    } else {
      startTimer()
      console.log('▶ Screen UNLOCKED — timer resumed')
    }
  })

  // ── Idle Detection ───────────────────────────────────────
  await onIdle((payload) => {
    if (payload.idle) {
      pauseTimer()
      saveState()
      console.log(`⏸ User IDLE for ${payload.seconds}s — timer paused`)
    } else {
      startTimer()
      console.log('▶ User ACTIVE — timer resumed')
    }
  })

  // ── System Suspend ───────────────────────────────────────
  await onSuspend(() => {
    pauseTimer()
    saveState()
    console.log('💤 System SUSPENDING — timer paused, state saved')
  })

  // ── System Resume ────────────────────────────────────────
  await onResume(() => {
    syncToServer()
    startTimer()
    console.log('☀️ System RESUMED — timer restarted, data synced')
  })
}

export async function teardownPowerMonitoring() {
  pauseTimer()
  await stop()
  console.log('🛑 Power monitoring stopped')
}

// ── One-shot idle time query (doesn't require start()) ──────
export async function logCurrentIdleTime() {
  const { seconds } = await getIdleTime()
  console.log(`User has been idle for ${seconds} seconds`)
}
