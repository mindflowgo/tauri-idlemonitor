// Minimal example: Just listen for screen lock/unlock events.
//
// This is the simplest possible usage of the idlemonitor plugin.

import { start, onLock } from 'tauri-plugin-idlemonitor-api'

await start()

await onLock(({ locked }) => {
  document.title = locked ? '🔒 Locked' : '🔓 Unlocked'
})
