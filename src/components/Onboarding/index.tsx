import { AnimatePresence, motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
// import { useAuthStore } from '../../stores/authStore'
import { saveOnboardingCompleted, updateConfig as saveConfig } from '../../lib/tauri'
import { OnboardingLayout } from './OnboardingLayout'
import { WelcomeStep } from './WelcomeStep'
// import { AccountStep } from './AccountStep'
import { ModeSelectStep } from './ModeSelectStep'
import { SttSetupStep } from './SttSetupStep'
import { LlmSetupStep } from './LlmSetupStep'
import { QuickTestStep } from './QuickTestStep'
import { DoneStep } from './DoneStep'
import { slideRight } from '../../lib/animations'

const TOTAL_STEPS = 6

export function Onboarding() {
  const { t } = useTranslation()
  const step = useAppStore((s) => s.onboardingStep)
  const setStep = useAppStore((s) => s.setOnboardingStep)
  const setOnboardingCompleted = useAppStore((s) => s.setOnboardingCompleted)
  const sttTestStatus = useAppStore((s) => s.sttTestStatus)
  const llmTestStatus = useAppStore((s) => s.llmTestStatus)
  const onboardingMode = useAppStore((s) => s.onboardingMode)
  // const setOnboardingMode = useAppStore((s) => s.setOnboardingMode)
  const updateConfig = useAppStore((s) => s.updateConfig)
  // const user = useAuthStore((s) => s.user)

  // Steps (Account step temporarily removed):
  // 0: Welcome, 1: ModeSelect, 2: SttSetup, 3: LlmSetup, 4: QuickTest, 5: Done
  const canNext = (() => {
    switch (step) {
      case 0:
        return true // Welcome — always
      // case 1 (Account) removed
      case 1:
        return onboardingMode !== null // Mode — need selection
      case 2:
        return sttTestStatus === 'success' // STT must pass (BYOK only)
      case 3:
        return llmTestStatus === 'success' // LLM must pass (BYOK only)
      case 4:
        return true // Quick test — optional
      case 5:
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
    // Account step removed:
    // { title: t('account.signIn'), subtitle: t('onboarding.signInSubtitle') },
    {
      title: t('onboarding.chooseModeTitle'),
      subtitle: t('onboarding.chooseModeSubtitle'),
    },
    {
      title: t('onboarding.speechRecognition'),
      subtitle: t('onboarding.speechRecognitionDesc'),
    },
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
      // Cloud mode: set providers BEFORE saving, then skip STT/LLM setup
      if (step === 1 && onboardingMode === 'cloud') {
        updateConfig({ stt_provider: 'cloud', llm_provider: 'cloud' })
        try {
          await saveConfig({ ...config, stt_provider: 'cloud', llm_provider: 'cloud' })
        } catch {
          // Best-effort save
        }
        setStep(4)
        return
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

      // If coming back from Quick Test in cloud mode, go back to Mode Select (step 1)
      if (step === 4 && onboardingMode === 'cloud') {
        setStep(1)
        return
      }

      // Account step removed — no skip-login back-navigation needed

      setStep(step - 1)
    }
  }

  const handleSkip = async () => {
    // Account step (step 1) removed — skip goes straight to completing onboarding
    // Original behavior: skip entire onboarding
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
          {/* Account step temporarily removed: {step === 1 && <AccountStep />} */}
          {step === 1 && <ModeSelectStep />}
          {step === 2 && <SttSetupStep />}
          {step === 3 && <LlmSetupStep />}
          {step === 4 && <QuickTestStep />}
          {step === 5 && <DoneStep />}
        </motion.div>
      </AnimatePresence>
    </OnboardingLayout>
  )
}
