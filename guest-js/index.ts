import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface LockPayload {
  locked: boolean
}

export interface IdlePayload {
  idle: boolean
  seconds?: number
}

export interface StartOptions {
  idleThresholdSecs?: number
}

export async function start(options?: StartOptions): Promise<void> {
  await invoke('plugin:idlemonitor|start', { options })
}

export async function stop(): Promise<void> {
  await invoke('plugin:idlemonitor|stop')
}

export async function getIdleTime(): Promise<{ seconds: number }> {
  return await invoke('plugin:idlemonitor|get_idle_time')
}

export async function onLock(
  handler: (payload: LockPayload) => void,
): Promise<UnlistenFn> {
  return listen<LockPayload>('power:lock', (e) => handler(e.payload))
}

export async function onIdle(
  handler: (payload: IdlePayload) => void,
): Promise<UnlistenFn> {
  return listen<IdlePayload>('power:idle', (e) => handler(e.payload))
}

export async function onSuspend(handler: () => void): Promise<UnlistenFn> {
  return listen('power:suspend', () => handler())
}

export async function onResume(handler: () => void): Promise<UnlistenFn> {
  return listen('power:resume', () => handler())
}
