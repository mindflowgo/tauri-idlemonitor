import {
  start,
  onLock,
  onIdle,
  onSuspend,
  onResume,
  getIdleTime,
} from 'tauri-plugin-idlemonitor-api'

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
  
  const timerEl = document.querySelector("#timer") as HTMLElement
  if (timerEl) {
    timerEl.style.opacity = "1.0"
  }

  // Update immediately, then start interval
  updateTimerDisplay()
  timerInterval = setInterval(() => {
    updateTimerDisplay()
  }, 1000)
}

function pauseTimer(reason: string) {
  updateStatus(reason, "#FF9800")
  if (timerInterval) {
    clearInterval(timerInterval)
    timerInterval = null
  }
  
  const timerEl = document.querySelector("#timer") as HTMLElement
  if (timerEl) {
    timerEl.style.opacity = "0.5"
  }
}

function updateTimerDisplay() {
  const timerEl = document.querySelector("#timer") as HTMLElement
  if (timerEl) {
    timerEl.textContent = new Date().toLocaleTimeString()
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
    const idleEl = document.querySelector("#idle-status") as HTMLElement
    if (payload.idle) {
      pauseTimer("Idle")
      logEvent(`⏸ User IDLE for ${payload.seconds}s — timer paused`)
      if (idleEl) idleEl.style.display = "block"
    } else {
      startTimer()
      logEvent('▶ User ACTIVE — timer resumed')
      if (idleEl) idleEl.style.display = "none"
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

  // Continuously poll the actual idle time from the OS
  setInterval(async () => {
    const { seconds } = await getIdleTime()
    const idleEl = document.querySelector("#idle-status")
    if (idleEl) {
      idleEl.textContent = `Currently Idle for: ${seconds}s`
    }
  }, 1000)
})
