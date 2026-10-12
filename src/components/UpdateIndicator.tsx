import { useLatestRelease } from '../lib/updateStatus'

// Kaliber R20: a found update is a quiet info line in the header tools that leads to About,
// never a warning colour. It never checks the network itself; it only shows what a check
// (at start, on About open or on the button) already found.
export function UpdateIndicator({ onOpenAbout }: { onOpenAbout: () => void }) {
  const release = useLatestRelease()
  if (!release) return null
  return (
    <button type="button" className="k-update-line" onClick={onOpenAbout}>
      Nová verzia {release.tag}
    </button>
  )
}
