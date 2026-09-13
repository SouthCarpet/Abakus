import { useLatestRelease } from '../lib/updateStatus'
import { Button } from './Button'

// Point 5: the global counterpart to Settings' quiet update line. It never
// checks the network itself. It only shows whatever Settings already found
// (or nothing, if the preference is off or no check ever ran).
export function UpdateIndicator({ onOpenSettings }: { onOpenSettings: () => void }) {
  const release = useLatestRelease()
  if (!release) return null
  return (
    <Button variant="ghost" className="k-update-indicator" onClick={onOpenSettings}>
      Nová verzia {release.tag}
    </Button>
  )
}
