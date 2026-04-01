import { useRef, useCallback, useEffect } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { useRecording } from '../../hooks/useRecording'
import { useCapsuleResize } from '../../hooks/useCapsuleResize'
import { CapsuleIdle } from './CapsuleIdle'
import { CapsuleRecording } from './CapsuleRecording'
import { CapsuleProcessing } from './CapsuleProcessing'
import { CapsulePolishing } from './CapsulePolishing'
import { CapsuleComplete } from './CapsuleComplete'
import { CapsuleError } from './CapsuleError'
import { CapsuleContextMenu } from './CapsuleContextMenu'

const DRAG_THRESHOLD = 5
// Must match TOAST_EXTRA_HEIGHT in useCapsuleResize (32px = 28px toast + 4px gap)
const TOAST_BOTTOM_OFFSET = 52 // 12px bottom padding + 36px pill + 4px gap

// Map target_lang codes to display names (localized name + region in parentheses)
const LANG_NAMES: Record<string, string> = {
  en: '英语（英国）',
  zh: '中文（简体）',
  ja: '日语（日本）',
  ko: '韩语（韩国）',
  fr: '法语（法国）',
  de: '德语（德国）',
  es: '西班牙语',
  pt: '葡萄牙语',
  ru: '俄语',
  ar: '阿拉伯语',
}

function getCapsuleState(pipelineState: string, hasError: boolean) {
  if (hasError) return 'error'
  return pipelineState
}

export function Capsule() {
  const { t } = useTranslation()
  const pipelineState = useAppStore((s) => s.pipelineState)
  const pipelineError = useAppStore((s) => s.pipelineError)
  const contextMenuOpen = useAppStore((s) => s.contextMenuOpen)
  const setContextMenuOpen = useAppStore((s) => s.setContextMenuOpen)
  const contextMenuReady = useAppStore((s) => s.contextMenuReady)
  const setContextMenuReady = useAppStore((s) => s.setContextMenuReady)
  const isTranslateSession = useAppStore((s) => s.isTranslateSession)
  const capsuleToast = useAppStore((s) => s.capsuleToast)
  const setCapsuleToast = useAppStore((s) => s.setCapsuleToast)
  const targetLang = useAppStore((s) => s.config.target_lang)
  const { startRecording, stopRecording, isRecording, isProcessing } = useRecording()

  const dragStart = useRef<{ x: number; y: number } | null>(null)
  const isDragging = useRef(false)

  useCapsuleResize()

  const hasError = pipelineError !== null
  const capsuleState = getCapsuleState(pipelineState, hasError)

  // Auto-clear the cancel toast after 2s
  useEffect(() => {
    if (capsuleToast === null) return
    const timer = setTimeout(() => setCapsuleToast(null), 2000)
    return () => clearTimeout(timer)
  }, [capsuleToast, setCapsuleToast])

  // Compute the toast message to show above the pill
  const toastMessage: string | null = (() => {
    if (capsuleToast) return capsuleToast
    if (isTranslateSession && (pipelineState === 'recording' || pipelineState === 'transcribing' || pipelineState === 'polishing')) {
      const langName = LANG_NAMES[targetLang] ?? targetLang
      return t('capsule.translatingTo', { lang: langName })
    }
    return null
  })()

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return
    dragStart.current = { x: e.clientX, y: e.clientY }
    isDragging.current = false
  }, [])

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!dragStart.current || isDragging.current) return
    const dx = e.clientX - dragStart.current.x
    const dy = e.clientY - dragStart.current.y
    if (Math.abs(dx) > DRAG_THRESHOLD || Math.abs(dy) > DRAG_THRESHOLD) {
      isDragging.current = true
      dragStart.current = null
      import('@tauri-apps/api/window')
        .then(({ getCurrentWindow }) => {
          getCurrentWindow()
            .startDragging()
            .catch(() => {})
        })
        .catch(() => {})
    }
  }, [])

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      if (e.button !== 0) return
      if (isDragging.current) {
        isDragging.current = false
        dragStart.current = null
        return
      }
      dragStart.current = null

      if (isRecording) {
        stopRecording()
      } else if (!isProcessing && !hasError && pipelineState === 'idle') {
        startRecording()
      }
    },
    [isRecording, isProcessing, hasError, pipelineState, startRecording, stopRecording],
  )

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    if (!contextMenuOpen) {
      setContextMenuOpen(true)
    }
  }

  const handleCloseMenu = () => {
    setContextMenuReady(false)
    setContextMenuOpen(false)
  }

  return (
    <div
      className="w-full h-full relative"
      style={{ background: 'transparent' }}
      onContextMenu={handleContextMenu}
    >
      {/* Toast label — floats above the pill */}
      <AnimatePresence>
        {toastMessage && (
          <motion.div
            key={toastMessage}
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 4 }}
            transition={{ duration: 0.18 }}
            style={{ position: 'absolute', bottom: `${TOAST_BOTTOM_OFFSET}px`, left: '12px' }}
            className="pointer-events-none"
          >
            <span
              className="inline-flex items-center px-2.5 py-1 rounded-full text-[11px] font-medium whitespace-nowrap"
              style={{
                color: '#ffffff',
                background: '#000000',
              }}
            >
              {toastMessage}
            </span>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Persistent outer shell — jelly capsule, anchored to bottom-left */}
      <motion.div
        className={`absolute bottom-3 left-3 rounded-full pointer-events-auto shrink-0 ${
          capsuleState === 'error'
            ? 'jelly-capsule-error'
            : capsuleState === 'idle'
              ? 'jelly-capsule text-neutral-700'
              : 'jelly-capsule-active text-white'
        }`}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <AnimatePresence mode="wait" initial={false}>
          <motion.div
            key={capsuleState}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
          >
            {capsuleState === 'idle' && <CapsuleIdle />}
            {capsuleState === 'recording' && <CapsuleRecording />}
            {capsuleState === 'transcribing' && <CapsuleProcessing />}
            {capsuleState === 'polishing' && <CapsulePolishing />}
            {capsuleState === 'outputting' && <CapsuleComplete />}
            {capsuleState === 'error' && <CapsuleError />}
          </motion.div>
        </AnimatePresence>
      </motion.div>

      {/* Context menu appears to the right of capsule */}
      {contextMenuOpen && contextMenuReady && (
        <div className="absolute bottom-3 left-3 ml-2" style={{ marginLeft: '4px' }}>
          <CapsuleContextMenu onClose={handleCloseMenu} />
        </div>
      )}
    </div>
  )
}
