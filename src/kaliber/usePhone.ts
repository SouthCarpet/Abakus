import { useEffect, useState } from 'react'

// Kaliber's phone breakpoint (R31): the same 599 px edge as kaliber.css and abakus.css.
const PHONE_MAX_WIDTH = 599

function isPhone(): boolean {
  return window.innerWidth <= PHONE_MAX_WIDTH
}

/** True while the window is a phone width. Follows a resize. */
export function usePhone(): boolean {
  const [phone, setPhone] = useState(isPhone)
  useEffect(() => {
    const onResize = () => setPhone(isPhone())
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])
  return phone
}
