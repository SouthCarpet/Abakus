import { Icon } from '../kaliber/icons'
import { useEffect } from 'react'
import { dismissToast, liveToasts, removeToast, useToasts } from '../kaliber/toast'
import { useEscapeLayer } from '../kaliber/escape'

// The toast region at the bottom right (Kaliber R12). A toast enters from its edge and leaves
// with opacity (kaliber.css); without an animation (reduced motion, tests) it leaves at once.
// Escape removes the newest toast (layer 2); Späť stays on Ctrl+Z after that.
export function Toasts() {
  const items = useToasts()
  const live = items.filter((item) => !item.leaving)
  useEscapeLayer('toast', live.length > 0, () => {
    const newest = liveToasts().at(-1)
    if (newest) dismissToast(newest.id)
  })
  useEffect(() => {
    for (const item of items) {
      if (!item.leaving) continue
      const el = document.querySelector(`[data-toast="${item.id}"]`)
      const name = el ? window.getComputedStyle(el).animationName : ''
      if (!name || name === 'none') removeToast(item.id)
    }
  }, [items])
  return (
    <div className="k-toast-region">
      {items.map((item) => (
        <div
          key={item.id}
          className={`k-well k-toast ${item.kind === 'error' ? 'k-toast--error ' : ''}${item.leaving ? 'is-leaving' : 'is-entering'}`}
          role="status"
          data-toast={item.id}
          onAnimationEnd={() => { if (item.leaving) removeToast(item.id) }}
        >
          <span className="k-toast-dot" aria-hidden="true">{item.kind === 'error' ? <Icon name="danger" small /> : null}</span>
          <span>{item.text}</span>
          {item.action ? (
            <button
              type="button"
              className="k-btn k-btn-ghost k-btn-auto k-toast-action"
              onClick={() => { item.action?.run(); dismissToast(item.id) }}
            >
              {item.action.label}
            </button>
          ) : null}
        </div>
      ))}
    </div>
  )
}
