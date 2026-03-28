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
    }, 2500)
    return () => clearTimeout(timer)
  }, [setPipelineError, resetRecording, setPipelineState])

  const message = pipelineError
    ? t(pipelineError, { defaultValue: pipelineError })
    : t('capsule.errorOccurred')

  return (
    <motion.div
      className="relative z-10 flex items-center gap-2 h-9 px-3"
      initial={{ opacity: 0, x: -4 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.3, ease: 'easeOut' }}
    >
      {/* White dot */}
      <motion.div className="w-2 h-2 rounded-full bg-white/80 flex-shrink-0" />
      <p className="text-[11px] text-white truncate flex-1">{message}</p>
    </motion.div>
  )
}
