import { useEffect, useRef, useState } from 'react'
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

// Size of the capsule window when the copy-ready dialog is floating in it
const COPY_DIALOG_WINDOW = { width: 380, height: 240 }

/**
 * Returns { capsuleHidden } — true after a copy dialog is dismissed, false once
 * recording resumes. While true the Capsule renders a transparent empty div so the
 * idle white circle never appears after the user clicks "Copy".
 */
export function useCapsuleResize(): { capsuleHidden: boolean } {
  const pipelineState = useAppStore((s) => s.pipelineState)
  const capsuleExpanded = useAppStore((s) => s.capsuleExpanded)
  const pipelineError = useAppStore((s) => s.pipelineError)
  const contextMenuOpen = useAppStore((s) => s.contextMenuOpen)
  const setContextMenuReady = useAppStore((s) => s.setContextMenuReady)
  const capsuleToast = useAppStore((s) => s.capsuleToast)
  const isTranslateSession = useAppStore((s) => s.isTranslateSession)
  const copyReadyText = useAppStore((s) => s.copyReadyText)

  const initialized = useRef(false)
  const prevWindowSize = useRef<{ width: number; height: number } | null>(null)
  const prevCopyReadyText = useRef<string | null>(null)

  // When true: Capsule renders nothing (transparent) until next recording starts.
  // Set after copy dialog dismissed; cleared when pipelineState goes non-idle.
  const [capsuleHidden, setCapsuleHidden] = useState(false)

  const hasError = pipelineError !== null
  const toastVisible = capsuleToast !== null || isTranslateSession

  // Clear hidden flag as soon as a recording (or any active pipeline state) starts.
  useEffect(() => {
    if (pipelineState !== 'idle') {
      setCapsuleHidden(false)
    }
  }, [pipelineState])

  useEffect(() => {
    // Cancellation flag: when deps change, React calls the cleanup which sets cancelled=true.
    // Every await in the async body checks this flag and aborts — prevents a stale
    // pipelineState effect (e.g. "outputting") from overwriting the copy-dialog size/position
    // after a newer "idle + copy_ready" effect has already set it correctly.
    let cancelled = false

    const wasCopyDialog = prevCopyReadyText.current !== null
    const isCopyDialog = copyReadyText !== null
    prevCopyReadyText.current = copyReadyText

    const size = getSizeForState(pipelineState, capsuleExpanded, hasError, contextMenuOpen, toastVisible)
    const windowWidth = isCopyDialog ? COPY_DIALOG_WINDOW.width : size.width + 24
    const windowHeight = isCopyDialog ? COPY_DIALOG_WINDOW.height : size.height + 24

    import('@tauri-apps/api/window')
      .then(async ({ getCurrentWindow, LogicalSize, LogicalPosition, currentMonitor, primaryMonitor }) => {
        const win = getCurrentWindow()

        // Helper: position window centered at screen bottom.
        // Uses currentMonitor() with primaryMonitor() fallback — currentMonitor() can return null
        // for hidden windows, which would leave the position unchanged and cause the dialog
        // to appear in the wrong location on subsequent invocations.
        const positionCenterBottom = async (w: number, h: number) => {
          try {
            const monitor = (await currentMonitor()) ?? (await primaryMonitor())
            if (cancelled) return
            if (monitor) {
              const sw = monitor.size.width / monitor.scaleFactor
              const sh = monitor.size.height / monitor.scaleFactor
              const x = Math.round(sw / 2 - w / 2)
              const y = Math.round(sh - h - 80)
              await win.setPosition(new LogicalPosition(x, y)).catch(() => {})
            }
          } catch (e) {
            console.warn('[Capsule] monitor error:', e)
          }
        }

        if (!initialized.current) {
          await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
          if (cancelled) return
          await positionCenterBottom(windowWidth, windowHeight)
          if (cancelled) return
          initialized.current = true
          prevWindowSize.current = { width: windowWidth, height: windowHeight }
          return
        }

        if (isCopyDialog) {
          // Show copy dialog centered at screen bottom regardless of where capsule was dragged
          await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
          if (cancelled) return
          await positionCenterBottom(windowWidth, windowHeight)
          if (cancelled) return
          await win.show().catch(() => {})
          if (cancelled) return
          prevWindowSize.current = { width: windowWidth, height: windowHeight }
          return
        }

        if (wasCopyDialog) {
          // Copy dialog dismissed — resize and reposition WHILE the window is still visible
          // (so currentMonitor() works), then hide. The Capsule component already renders a
          // transparent div because setCapsuleHidden(true) triggers a synchronous React re-render.
          setCapsuleHidden(true)
          await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
          if (cancelled) return
          await positionCenterBottom(windowWidth, windowHeight)
          if (cancelled) return
          await win.hide().catch(() => {})
          if (cancelled) return
          prevWindowSize.current = { width: windowWidth, height: windowHeight }
          return
        }

        // Normal resize: left edge fixed, bottom edge anchored so the pill stays in place
        const prev = prevWindowSize.current
        if (prev) {
          const pos = await win.outerPosition().catch(() => null)
          if (cancelled) return
          if (pos) {
            const monitor = await currentMonitor()
            if (cancelled) return
            const scale = monitor?.scaleFactor ?? 1
            const oldLeftX = pos.x / scale
            const oldBottomY = pos.y / scale + prev.height
            const newX = Math.round(oldLeftX)
            const newY = Math.round(oldBottomY - windowHeight)
            await win.setPosition(new LogicalPosition(newX, newY)).catch(() => {})
            if (cancelled) return
            await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
            if (cancelled) return
          } else {
            await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
            if (cancelled) return
          }
        } else {
          await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
          if (cancelled) return
        }

        prevWindowSize.current = { width: windowWidth, height: windowHeight }

        if (pipelineState === 'idle') {
          // Hide the capsule when the pipeline goes idle. Previously this was done in Rust's
          // set_state(Idle) → capsule.hide(), but that ran synchronously before the frontend's
          // win.show() for the copy dialog could execute, causing a race condition that left
          // the copy dialog invisible on the second invocation.
          await win.hide().catch(() => {})
        } else {
          import('@tauri-apps/api/core')
            .then(({ invoke }) => {
              invoke('bring_capsule_to_front').catch(() => {})
            })
            .catch(() => {})

          if (contextMenuOpen) {
            setContextMenuReady(true)
          }
        }
      })
      .catch(() => {})

    return () => {
      cancelled = true
    }
  }, [pipelineState, capsuleExpanded, hasError, contextMenuOpen, setContextMenuReady, toastVisible, copyReadyText])

  return { capsuleHidden }
}
