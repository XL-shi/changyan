import { AnimatePresence, motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
// import { useAuthStore } from '../../stores/authStore'
import { saveOnboardingCompleted, updateConfig as saveConfig } from '../../lib/tauri'
import { OnboardingLayout } from './OnboardingLayout'
import { WelcomeStep } from './WelcomeStep'
// import { AccountStep } from './AccountStep'
// ModeSelectStep removed — defaults to BYOK (API key setup)
import { LlmSetupStep } from './LlmSetupStep'
import { QuickTestStep } from './QuickTestStep'
import { DoneStep } from './DoneStep'
import { slideRight } from '../../lib/animations'

const TOTAL_STEPS = 4

export function Onboarding() {
  const { t } = useTranslation()
  const step = useAppStore((s) => s.onboardingStep)
  const setStep = useAppStore((s) => s.setOnboardingStep)
  const setOnboardingCompleted = useAppStore((s) => s.setOnboardingCompleted)
  const llmTestStatus = useAppStore((s) => s.llmTestStatus)
  const updateConfig = useAppStore((s) => s.updateConfig)

  // Steps: 0: Welcome, 1: LlmSetup, 2: QuickTest, 3: Done
  const canNext = (() => {
    switch (step) {
      case 0:
        return true // Welcome — always
      case 1:
        return llmTestStatus === 'success' // LLM must pass
      case 2:
        return true // Quick test — optional
      case 3:
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
    {
      title: t('onboarding.tryItOut'),
      subtitle: t('onboarding.tryItOutDesc'),
    },
    { title: t('onboarding.setupComplete'), subtitle: undefined },
  ]

  const config = useAppStore((s) => s.config)

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
      await saveConfig(config)
      await saveOnboardingCompleted()
      setOnboardingCompleted(true)
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
    await saveConfig(config)
    await saveOnboardingCompleted()
    setOnboardingCompleted(true)
  }

  return (
    <OnboardingLayout
      step={step}
      totalSteps={TOTAL_STEPS}
      title={titles[step].title}
      subtitle={titles[step].subtitle}
      canNext={canNext}
      canBack={step > 0}
      nextLabel={step === TOTAL_STEPS - 1 ? t('onboarding.getStarted') : t('onboarding.next')}
      onNext={handleNext}
      onBack={handleBack}
      onSkip={handleSkip}
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
          {step === 2 && <QuickTestStep />}
          {step === 3 && <DoneStep />}
        </motion.div>
      </AnimatePresence>
    </OnboardingLayout>
  )
}
