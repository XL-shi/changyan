import { useEffect, useRef } from 'react'
import { useReducedMotion } from 'framer-motion'
import { useAppStore } from '../../stores/appStore'

const BAR_COUNT = 9
const MIN_HEIGHT = 3
const MAX_HEIGHT = 18

export function Waveform() {
  const barsRef = useRef<(HTMLDivElement | null)[]>([])
  const rafRef = useRef<number>(0)
  const reduced = useReducedMotion()

  useEffect(() => {
    if (reduced) {
      // Static bars at mid-height when reduced motion is preferred
      barsRef.current.forEach((bar) => {
        if (!bar) return
        bar.style.height = `${(MIN_HEIGHT + MAX_HEIGHT) / 2}px`
        bar.style.opacity = '0.7'
      })
      return
    }

    const animate = () => {
      const volume = useAppStore.getState().audioVolume
      // Use a stronger sine wave so bars are visibly animated even at near-zero volume.
      // Each bar gets its own phase offset for a natural ripple effect.
      barsRef.current.forEach((bar, i) => {
        if (!bar) return
        const wave = Math.sin(Date.now() / 160 + i * 1.1) * 0.35
        const normalized = Math.max(0.08, Math.min(1, volume + wave))
        const height = MIN_HEIGHT + (MAX_HEIGHT - MIN_HEIGHT) * normalized
        const opacity = 0.6 + normalized * 0.4
        bar.style.height = `${height}px`
        bar.style.opacity = `${opacity}`
      })
      rafRef.current = requestAnimationFrame(animate)
    }

    rafRef.current = requestAnimationFrame(animate)
    return () => cancelAnimationFrame(rafRef.current)
  }, [reduced])

  return (
    <div className="flex items-center justify-center gap-[3px] h-4">
      {Array.from({ length: BAR_COUNT }).map((_, i) => (
        <div
          key={i}
          ref={(el) => {
            barsRef.current[i] = el
          }}
          style={{
            width: '3px',
            height: `${MIN_HEIGHT}px`,
            background: 'rgba(255,255,255,0.9)',
            borderRadius: '9999px',
            opacity: 0.6,
            flexShrink: 0,
            transition: 'height 75ms ease-out, opacity 75ms ease-out',
          }}
        />
      ))}
    </div>
  )
}
