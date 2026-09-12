import { api } from '../api'

// A Settings remount must wait for the preceding screen's write. Only the
// in-flight operation is shared; every read still gets the value from SQLite.
let pendingWrite: Promise<void> = Promise.resolve()

export async function readUpdatePreference(): Promise<boolean> {
  await pendingWrite
  return api.getCheckUpdates()
}

export function saveUpdatePreference(on: boolean): Promise<void> {
  const write = api.setCheckUpdates(on)
  // A failed write still lets the next screen read the last saved value.
  // Return the original promise so the initiating screen reports the error.
  pendingWrite = write.catch(() => {})
  return write
}
