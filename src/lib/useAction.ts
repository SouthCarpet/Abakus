import { useCallback, useRef, useState } from 'react'

/** Serialize user mutations and keep failures visible at their owning screen. */
export function useAction() {
  const pending = useRef(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const run = useCallback(async (action: () => Promise<void>) => {
    if (pending.current) return
    pending.current = true
    setBusy(true)
    setError('')
    try { await action() }
    catch (e) { setError(String(e)) }
    finally { pending.current = false; setBusy(false) }
  }, [])
  return { busy, error, setError, run }
}
