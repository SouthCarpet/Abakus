import { useSyncExternalStore } from 'react'
import type { Release } from '../api'

// Point 5: the last update result Settings already knows, shared with the
// rest of the app so a global indicator can show it. This module never calls
// the network itself. It only stores whatever Settings (the one screen that
// runs `check_update_now`) already found, and it starts empty until then.
let latestRelease: Release | null = null
const listeners = new Set<() => void>()

export function getLatestRelease(): Release | null {
  return latestRelease
}

export function setLatestRelease(release: Release | null): void {
  latestRelease = release
  for (const listen of listeners) listen()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function useLatestRelease(): Release | null {
  return useSyncExternalStore(subscribe, getLatestRelease)
}
