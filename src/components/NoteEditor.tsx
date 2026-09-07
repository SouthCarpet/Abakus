import { useEffect, useRef, useState } from 'react'
import { api } from '../api'
import { Button } from './Button'

const NOTE_MAX_CODE_POINTS = 2000

type NoteStatus = 'idle' | 'saving' | 'saved' | 'error'

export function NoteEditor({ txId, initialNote, onSaved }: {
  txId: number
  initialNote: string
  onSaved: () => Promise<void>
}) {
  const [draft, setDraft] = useState(initialNote)
  const [status, setStatus] = useState<NoteStatus>('idle')
  const [error, setError] = useState('')
  const [refreshError, setRefreshError] = useState('')
  const pending = useRef(false)
  const mounted = useRef(true)
  // Kept current every render (not just in the effect below) so a save
  // continuation resuming after txId already changed on this same mounted
  // instance can tell it is stale before touching status or error state.
  const activeTxId = useRef(txId)
  activeTxId.current = txId
  // The text this component instance last actually persisted for this txId,
  // so a retry after a refresh failure (same unchanged draft, no further
  // edits) only retries the refresh, never repeats an already-successful
  // write. Starts at null (not initialNote): the first save for a freshly
  // opened row always writes, even if the user re-types the same text.
  const lastSaved = useRef<string | null>(null)

  useEffect(() => {
    mounted.current = true
    return () => { mounted.current = false }
  }, [])

  // A new transaction id means a different note entirely; re-sync from it
  // and release any save lock a still-settling previous save left behind.
  // The same id re-rendering with a fresh initialNote (a background refresh)
  // must never stomp on text the user is still mid-edit for that same row.
  useEffect(() => {
    setDraft(initialNote); setStatus('idle'); setError(''); setRefreshError('')
    pending.current = false
    lastSaved.current = null
  }, [txId])

  const codePoints = [...draft].length
  const overLimit = codePoints > NOTE_MAX_CODE_POINTS

  async function save() {
    if (pending.current || overLimit) return
    const savingFor = txId
    const stillCurrent = () => mounted.current && activeTxId.current === savingFor
    pending.current = true
    setStatus('saving'); setError(''); setRefreshError('')
    if (draft !== lastSaved.current) {
      try {
        await api.saveTransactionNote(savingFor, draft)
      } catch (e) {
        if (activeTxId.current === savingFor) pending.current = false
        if (stillCurrent()) { setStatus('error'); setError(String(e)) }
        return
      }
      lastSaved.current = draft
    }
    if (stillCurrent()) setStatus('saved')
    try {
      await onSaved()
    } catch (e) {
      if (stillCurrent()) setRefreshError(String(e))
    } finally {
      if (activeTxId.current === savingFor) pending.current = false
    }
  }

  return (
    <div className="k-field">
      <label className="k-field-label" htmlFor={`note-${txId}`}>Poznámka</label>
      <textarea
        id={`note-${txId}`}
        aria-label={`Poznámka k transakcii ${txId}`}
        className="k-input k-well"
        value={draft}
        disabled={status === 'saving'}
        onChange={(e) => { setDraft(e.target.value); setStatus('idle') }}
      />
      <div className="k-row">
        <Button variant="secondary" disabled={status === 'saving' || overLimit} onClick={() => void save()}>
          Uložiť poznámku
        </Button>
        <span className="k-field-label">{codePoints} / {NOTE_MAX_CODE_POINTS} znakov</span>
      </div>
      {overLimit ? <p role="alert">Poznámka je príliš dlhá, skráťte ju.</p> : null}
      {status === 'saving' ? <p role="status">Ukladá sa...</p> : null}
      {status === 'saved' && !refreshError ? <p role="status">Poznámka uložená.</p> : null}
      {status === 'error' ? <p role="alert">{error}</p> : null}
      {refreshError ? <p role="alert">Poznámka je uložená, obnovenie zoznamu zlyhalo: {refreshError}</p> : null}
    </div>
  )
}
