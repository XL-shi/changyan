import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { motion } from 'framer-motion'
import { useAppStore } from '../../stores/appStore'

export function CapsuleError() {
  const { t } = useTranslation()
  const pipelineError = useAppStore((s) => s.pipelineError)
  const setPipelineError = useAppStore((s) => s.setPipelineError)
  const setPipelineState = useAppStore((s) => s.setPipelineState)
  const resetRecording = useAppStore((s) => s.resetRecording)

  useEffect(() => {
    const timer = setTimeout(() => {
      setPipelineError(null)
      resetRecording()
      setPipelineState('idle')
    }, 4000)
    return () => clearTimeout(timer)
  }, [setPipelineError, resetRecording, setPipelineState])

  const message = pipelineError
    ? t(pipelineError, { defaultValue: pipelineError })
    : t('capsule.errorOccurred')

  return (
    <motion.div
      className="relative z-10 flex flex-col gap-1.5 px-3.5 py-3 w-[300px]"
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.2, ease: 'easeOut' }}
    >
      <div className="flex items-center gap-2">
        <motion.div
          className="w-2 h-2 rounded-full bg-white flex-shrink-0"
          animate={{ opacity: [1, 0.4, 1] }}
          transition={{ duration: 1.5, repeat: Infinity }}
        />
        <span className="text-[11px] font-semibold text-white/70 uppercase tracking-wider">
          Error
        </span>
      </div>
      <p className="text-[12px] text-white leading-snug break-words whitespace-normal">{message}</p>
    </motion.div>
  )
}
