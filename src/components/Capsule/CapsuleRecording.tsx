import { motion, useReducedMotion } from 'framer-motion'
import { X, Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { forceIdle, stopRecording } from '../../lib/tauri'
import { useAppStore } from '../../stores/appStore'
import { Waveform } from './Waveform'

export function CapsuleRecording() {
  const { t } = useTranslation()
  const reduced = useReducedMotion()
  const setCapsuleToast = useAppStore((s) => s.setCapsuleToast)
  const resetRecording = useAppStore((s) => s.resetRecording)
  const setPipelineState = useAppStore((s) => s.setPipelineState)

  const handleCancel = async (e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      await forceIdle()
    } catch (err) {
      console.error('force_idle failed:', err)
    }
    resetRecording()
    setPipelineState('idle')
    setCapsuleToast(t('capsule.recordingCancelled', '转录已取消'))
  }

  const handleConfirm = async (e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      await stopRecording()
    } catch (err) {
      console.error('stop_recording failed:', err)
    }
  }

  return (
    <motion.div className="relative z-10 flex items-center justify-between h-9 px-3 w-[200px]" initial={false}>
      <motion.button
        onClick={handleCancel}
        aria-label={t('capsule.cancelRecording', 'Cancel recording')}
        className="w-6 h-6 rounded-full bg-white flex items-center justify-center border-none cursor-pointer flex-shrink-0"
        whileHover={reduced ? undefined : { scale: 1.1 }}
        whileTap={reduced ? undefined : { scale: 0.9 }}
      >
        <X size={12} className="text-neutral-900" strokeWidth={2.5} />
      </motion.button>

      <Waveform />

      <motion.button
        onClick={handleConfirm}
        aria-label={t('capsule.confirmRecording', 'Stop and transcribe')}
        className="w-6 h-6 rounded-full bg-white flex items-center justify-center border-none cursor-pointer flex-shrink-0"
        whileHover={reduced ? undefined : { scale: 1.1 }}
        whileTap={reduced ? undefined : { scale: 0.9 }}
      >
        <Check size={12} className="text-neutral-900" strokeWidth={2.5} />
      </motion.button>
    </motion.div>
  )
}
