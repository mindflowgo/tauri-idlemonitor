import {
  start,
  onLock,
  onIdle,
  onSuspend,
  onResume,
  getIdleTime,
} from 'tauri-plugin-idlemonitor-api'

let timerSeconds = 0
let timerInterval: ReturnType<typeof setInterval> | null = null

function logEvent(msg: string) {
  const eventsEl = document.querySelector("#events")
  if (eventsEl) {
    const div = document.createElement("div")
    const now = new Date().toLocaleTimeString()
    div.textContent = `[${now}] ${msg}`
    eventsEl.appendChild(div)
    eventsEl.scrollTop = eventsEl.scrollHeight
  }
}

function updateStatus(status: string, color: string) {
  const statusEl = document.querySelector("#status") as HTMLElement
  if (statusEl) {
    statusEl.textContent = `Status: ${status}`
    statusEl.style.color = color
  }
}

function startTimer() {
  if (timerInterval) return
  updateStatus("Active", "#4CAF50")
  timerInterval = setInterval(() => {
    timerSeconds++
    updateTimerDisplay()
  }, 1000)
}

function pauseTimer(reason: string) {
  updateStatus(reason, "#FF9800")
  if (timerInterval) {
    clearInterval(timerInterval)
    timerInterval = null
  }
}

function updateTimerDisplay() {
  const mins = Math.floor(timerSeconds / 60)
  const secs = timerSeconds % 60
  const timerEl = document.querySelector("#timer")
  if (timerEl) {
    timerEl.textContent = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
  }
}

async function setupIdleMonitoring() {
  logEvent('Starting idle monitoring (10s threshold for demo)...')
  // Use a short threshold for demo purposes
  await start({ idleThresholdSecs: 10 })
  startTimer()
  logEvent('✅ Idle monitoring started')

  await onLock((payload) => {
    if (payload.locked) {
      pauseTimer("Locked")
      logEvent('⏸ Screen LOCKED — timer paused')
    } else {
      startTimer()
      logEvent('▶ Screen UNLOCKED — timer resumed')
    }
  })

  await onIdle((payload) => {
    if (payload.idle) {
      pauseTimer("Idle")
      logEvent(`⏸ User IDLE for ${payload.seconds}s — timer paused`)
    } else {
      startTimer()
      logEvent('▶ User ACTIVE — timer resumed')
    }
  })

  await onSuspend(() => {
    pauseTimer("Suspended")
    logEvent('💤 System SUSPENDING — timer paused')
  })

  await onResume(() => {
    startTimer()
    logEvent('☀️ System RESUMED — timer restarted')
  })
}

window.addEventListener("DOMContentLoaded", () => {
  setupIdleMonitoring()

  document.querySelector("#check-idle")?.addEventListener("click", async () => {
    const { seconds } = await getIdleTime()
    logEvent(`ℹ️ Current idle time: ${seconds} seconds`)
  })
})
