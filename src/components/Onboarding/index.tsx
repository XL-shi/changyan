import { AnimatePresence, motion } from 'framer-motion'
import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
// import { useAuthStore } from '../../stores/authStore'
import {
  saveOnboardingCompleted,
  updateConfig as saveConfig,
  getSenseVoiceModelStatus,
  downloadSenseVoiceModel,
  type ModelDownloadProgress,
} from '../../lib/tauri'
import { OnboardingLayout } from './OnboardingLayout'
import { WelcomeStep } from './WelcomeStep'
// import { AccountStep } from './AccountStep'
// ModeSelectStep removed — defaults to BYOK (API key setup)
import { LlmSetupStep } from './LlmSetupStep'
import { DoneStep } from './DoneStep'
import { slideRight } from '../../lib/animations'

const TOTAL_STEPS = 3

export function Onboarding() {
  const { t } = useTranslation()
  const step = useAppStore((s) => s.onboardingStep)
  const setStep = useAppStore((s) => s.setOnboardingStep)
  const setOnboardingCompleted = useAppStore((s) => s.setOnboardingCompleted)
  const llmTestStatus = useAppStore((s) => s.llmTestStatus)
  const updateConfig = useAppStore((s) => s.updateConfig)
  const [isFinalizing, setIsFinalizing] = useState(false)
  const [modelDownloadProgress, setModelDownloadProgress] = useState(0)
  const [modelDownloadError, setModelDownloadError] = useState<string | null>(null)

  // Steps: 0: Welcome, 1: LlmSetup, 2: Done
  const canNext = (() => {
    switch (step) {
      case 0:
        return true // Welcome — always
      case 1:
        return llmTestStatus === 'success' // LLM must pass
      case 2:
        return true // Done
      default:
        return false
    }
  })()

  const titles = [
    {
      title: t('onboarding.welcomeTitle'),
      subtitle: t('onboarding.welcomeSubtitle'),
    },
    // STT step removed — STT is built-in local SenseVoice Small
    {
      title: t('onboarding.aiPolish'),
      subtitle: t('onboarding.aiPolishDesc'),
    },
    { title: t('onboarding.setupComplete'), subtitle: undefined },
  ]

  const config = useAppStore((s) => s.config)

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined
    let unlistenComplete: (() => void) | undefined

    listen<ModelDownloadProgress>('model:download-progress', (e) => {
      setModelDownloadProgress(e.payload.percent)
    }).then((fn) => {
      unlistenProgress = fn
    })

    listen('model:download-complete', () => {
      setModelDownloadProgress(100)
    }).then((fn) => {
      unlistenComplete = fn
    })

    return () => {
      unlistenProgress?.()
      unlistenComplete?.()
    }
  }, [])

  const completeOnboarding = async () => {
    setIsFinalizing(true)
    setModelDownloadError(null)
    try {
      await saveConfig(config)

      if (config.stt_provider === 'sensevoice-local') {
        const status = await getSenseVoiceModelStatus()
        if (!status.isDownloaded) {
          setModelDownloadProgress(0)
          await downloadSenseVoiceModel()
        }
      }

      await saveOnboardingCompleted()
      setOnboardingCompleted(true)
    } catch (error) {
      setModelDownloadError(error instanceof Error ? error.message : String(error))
    } finally {
      setIsFinalizing(false)
    }
  }

  const handleNext = async () => {
    if (step < TOTAL_STEPS - 1) {
      // Entering LlmSetup: set provider to BYOK defaults
      if (step === 0) {
        updateConfig({ stt_provider: 'sensevoice-local' })
      }

      try {
        await saveConfig(config)
      } catch {
        // Best-effort save — continue navigation even if save fails
      }

      setStep(step + 1)
    } else {
      await completeOnboarding()
    }
  }

  const handleBack = async () => {
    if (step > 0) {
      try {
        await saveConfig(config)
      } catch {
        // Best-effort save
      }

      setStep(step - 1)
    }
  }

  const handleSkip = async () => {
    await completeOnboarding()
  }

  return (
    <OnboardingLayout
      step={step}
      totalSteps={TOTAL_STEPS}
      title={titles[step].title}
      subtitle={titles[step].subtitle}
      canNext={canNext && !isFinalizing}
      canBack={step > 0}
      nextLabel={
        step === TOTAL_STEPS - 1
          ? isFinalizing && config.stt_provider === 'sensevoice-local'
            ? `${t('settings.downloadingModel')} ${Math.round(modelDownloadProgress)}%`
            : t('onboarding.getStarted')
          : t('onboarding.next')
      }
      onNext={handleNext}
      onBack={handleBack}
      onSkip={isFinalizing ? undefined : handleSkip}
    >
      <AnimatePresence mode="wait">
        <motion.div
          key={step}
          variants={slideRight}
          initial="initial"
          animate="animate"
          exit="exit"
          transition={{ duration: 0.2 }}
        >
          {step === 0 && <WelcomeStep />}
          {/* STT step removed — STT is built-in local SenseVoice Small */}
          {step === 1 && <LlmSetupStep />}
          {step === 2 && (
            <DoneStep
              isFinalizing={isFinalizing}
              modelDownloadProgress={modelDownloadProgress}
              modelDownloadError={modelDownloadError}
              showModelDownloadStatus={config.stt_provider === 'sensevoice-local'}
            />
          )}
        </motion.div>
      </AnimatePresence>
    </OnboardingLayout>
  )
}
