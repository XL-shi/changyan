import { useEffect, useRef } from 'react'
import { useAppStore, type PipelineState } from '../stores/appStore'

// Extra height added to the window when a toast label is shown above the pill
const TOAST_EXTRA_HEIGHT = 32

interface CapsuleSize {
  width: number
  height: number
}

function getSizeForState(
  state: PipelineState,
  expanded: boolean,
  hasError: boolean,
  contextMenuOpen: boolean,
  toastVisible: boolean,
): CapsuleSize {
  if (contextMenuOpen) return { width: 220, height: 220 }
  if (hasError) return { width: 200, height: 36 }
  if (expanded) return { width: 220, height: 90 }

  let base: CapsuleSize
  switch (state) {
    case 'idle':
      base = { width: 36, height: 36 }
      break
    case 'recording':
      base = { width: 200, height: 36 }
      break
    case 'transcribing':
    case 'polishing':
      base = { width: 220, height: 36 }
      break
    case 'outputting':
      base = { width: 120, height: 36 }
      break
    default:
      base = { width: 36, height: 36 }
  }

  if (toastVisible) {
    return {
      width: Math.max(base.width, 180),
      height: base.height + TOAST_EXTRA_HEIGHT,
    }
  }
  return base
}

export function useCapsuleResize() {
  const pipelineState = useAppStore((s) => s.pipelineState)
  const capsuleExpanded = useAppStore((s) => s.capsuleExpanded)
  const pipelineError = useAppStore((s) => s.pipelineError)
  const contextMenuOpen = useAppStore((s) => s.contextMenuOpen)
  const setContextMenuReady = useAppStore((s) => s.setContextMenuReady)
  const capsuleToast = useAppStore((s) => s.capsuleToast)
  const isTranslateSession = useAppStore((s) => s.isTranslateSession)
  const initialized = useRef(false)
  const prevWindowSize = useRef<{ width: number; height: number } | null>(null)

  const hasError = pipelineError !== null
  const toastVisible = capsuleToast !== null || isTranslateSession

  useEffect(() => {
    const size = getSizeForState(pipelineState, capsuleExpanded, hasError, contextMenuOpen, toastVisible)
    const windowWidth = size.width + 24
    const windowHeight = size.height + 24

    import('@tauri-apps/api/window')
      .then(async ({ getCurrentWindow, LogicalSize, LogicalPosition, currentMonitor }) => {
        const win = getCurrentWindow()

        if (!initialized.current) {
          await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
          try {
            const monitor = await currentMonitor()
            console.log('[Capsule] currentMonitor:', monitor
              ? `${monitor.size.width}x${monitor.size.height} @${monitor.scaleFactor}x pos=(${monitor.position.x},${monitor.position.y})`
              : 'null')
            if (monitor) {
              const sw = monitor.size.width / monitor.scaleFactor
              const sh = monitor.size.height / monitor.scaleFactor
              const x = Math.round(sw / 2 - windowWidth / 2)
              const y = Math.round(sh - windowHeight - 80)
              console.log(`[Capsule] initial position: x=${x} y=${y} size=${windowWidth}x${windowHeight}`)
              await win.setPosition(new LogicalPosition(x, y)).catch(() => {})
            }
            const { invoke } = await import('@tauri-apps/api/core')
            console.log('[Capsule] invoking bring_capsule_to_front (first mount)')
            await invoke('bring_capsule_to_front').catch(() => win.show().catch(() => {}))
            const pos = await win.outerPosition().catch(() => null)
            const sz = await win.outerSize().catch(() => null)
            console.log(`[Capsule] after show: outerPos=${JSON.stringify(pos)} outerSize=${JSON.stringify(sz)}`)
            invoke<string>('debug_capsule_window').then((info) => {
              console.log('[Capsule] native state:', info)
            }).catch(() => {})
          } catch (e) {
            console.warn('[Capsule] monitor error:', e)
            await win.show().catch(() => {})
          }
          initialized.current = true
          prevWindowSize.current = { width: windowWidth, height: windowHeight }
          return
        }

        // Subsequent resizes: left edge fixed, bottom edge anchored so the pill
        // stays in place when the window grows upward to reveal the toast.
        const prev = prevWindowSize.current
        if (prev) {
          const pos = await win.outerPosition().catch(() => null)
          if (pos) {
            const monitor = await currentMonitor()
            const scale = monitor?.scaleFactor ?? 1
            const oldLeftX = pos.x / scale
            // Anchor to bottom: pill position on screen stays constant
            const oldBottomY = pos.y / scale + prev.height
            const newX = Math.round(oldLeftX)
            const newY = Math.round(oldBottomY - windowHeight)
            await win.setPosition(new LogicalPosition(newX, newY)).catch(() => {})
            await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
          } else {
            await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
          }
        } else {
          await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
        }

        prevWindowSize.current = { width: windowWidth, height: windowHeight }

        import('@tauri-apps/api/core').then(({ invoke }) => {
          console.log(`[Capsule] invoking bring_capsule_to_front (state=${pipelineState})`)
          invoke('bring_capsule_to_front').catch(() => {})
        }).catch(() => {})

        if (contextMenuOpen) {
          setContextMenuReady(true)
        }
      })
      .catch(() => {})
  }, [pipelineState, capsuleExpanded, hasError, contextMenuOpen, setContextMenuReady, toastVisible])

  return getSizeForState(pipelineState, capsuleExpanded, hasError, contextMenuOpen, toastVisible)
}
