import { X, Check } from 'lucide-react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { spring } from '../../lib/animations'

interface Props {
  step: number
  totalSteps: number
  title: string
  subtitle?: string
  description?: string
  canNext: boolean
  canBack: boolean
  nextLabel?: string
  onNext: () => void
  onBack: () => void
  onSkip?: () => void
  children: React.ReactNode
}

export function OnboardingLayout({
  step,
  totalSteps,
  title,
  subtitle,
  description,
  canNext,
  canBack,
  nextLabel,
  onNext,
  onBack,
  onSkip,
  children,
}: Props) {
  const { t } = useTranslation()

  const handleClose = () => {
    import('@tauri-apps/api/core')
      .then(({ invoke }) => invoke('plugin:process|exit', { code: 0 }))
      .catch(() => {})
  }

  // Short labels for each step in the left panel
  const stepLabelKeys = ['onboarding.step_welcome', 'onboarding.step_done']

  return (
    <div className="w-full h-full flex flex-col bg-bg-primary text-text-primary">
      {/* Close button */}
      <div className="absolute top-3 right-3 z-20" data-tauri-drag-region>
        <button
          onClick={handleClose}
          className="p-1.5 rounded hover:bg-bg-tertiary transition-colors bg-transparent border-none cursor-pointer text-text-tertiary hover:text-text-primary"
          aria-label="Close"
        >
          <X size={13} strokeWidth={2} />
        </button>
      </div>

      <div className="flex-1 flex min-h-0">
        {/* ── Left: Brand + Step list ── */}
        <aside className="w-[220px] bg-bg-secondary border-r border-border flex flex-col shrink-0">
          {/* Brand */}
          <div className="px-6 pt-8 pb-6" data-tauri-drag-region>
            <h1 className="cy-mark text-[22px] text-text-primary">{t('app.name')}</h1>
            <p
              className="mt-2 text-[11px] text-text-secondary tracking-[0.08em] uppercase"
              style={{ fontFamily: 'var(--font-display)' }}
            >
              {t('app.tagline')}
            </p>
          </div>

          <div className="cy-rule" />

          {/* Steps */}
          <nav className="flex-1 pt-1">
            {Array.from({ length: totalSteps }).map((_, i) => {
              const isCurrent = i === step
              const isDone = i < step
              const labelKey = stepLabelKeys[i] ?? `Step ${i + 1}`

              return (
                <div key={i} className="relative">
                  {isCurrent && (
                    <motion.div
                      layoutId="onboarding-fence"
                      className="absolute inset-0 pointer-events-none"
                      style={{
                        borderTop: '1px solid var(--color-accent)',
                        borderBottom: '1px solid var(--color-accent)',
                      }}
                      transition={spring.snappy}
                    />
                  )}
                  <div
                    className={`flex items-center gap-3 px-6 py-3.5 ${
                      isCurrent
                        ? 'text-text-primary'
                        : isDone
                          ? 'text-text-secondary'
                          : 'text-text-secondary'
                    }`}
                  >
                    <span
                      className="text-[11px] w-5 shrink-0 text-text-tertiary"
                      style={{ fontFamily: 'var(--font-display)' }}
                    >
                      {String(i + 1).padStart(2, '0')}
                    </span>
                    {isDone ? (
                      <Check size={12} strokeWidth={2.5} />
                    ) : (
                      <div
                        className={`w-3 h-3 rounded-full border ${isCurrent ? 'border-text-primary bg-text-primary' : 'border-text-tertiary'}`}
                      />
                    )}
                    <span className={`text-[13px] ${isCurrent ? 'font-medium' : ''}`}>
                      {t(labelKey)}
                    </span>
                  </div>
                </div>
              )
            })}
          </nav>
        </aside>

        {/* ── Right: Content ── */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Title */}
          <div className="px-8 pt-7 pb-5 border-b border-border">
            <h2 className="text-[20px] font-semibold text-text-primary leading-snug">{title}</h2>
            {description && (
              <p className="text-[13px] text-text-secondary mt-3 leading-relaxed border-l-2 border-border-strong pl-3 italic">
                {description}
              </p>
            )}
            {subtitle && (
              <p className="text-[13px] text-text-tertiary mt-3 leading-relaxed">{subtitle}</p>
            )}
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto px-8 py-6">{children}</div>

          {/* Navigation */}
          <div className="flex items-center justify-between px-8 py-4 border-t border-border">
            <motion.button
              onClick={onBack}
              disabled={!canBack}
              whileTap={canBack ? { scale: 0.97 } : undefined}
              transition={spring.snappy}
              className="text-[13px] text-text-secondary hover:text-text-primary bg-transparent border-none cursor-pointer disabled:opacity-0 disabled:cursor-default transition-colors"
            >
              {t('onboarding.back')}
            </motion.button>

            <div className="flex items-center gap-3">
              {onSkip && (
                <button
                  onClick={onSkip}
                  className="text-[12px] text-text-tertiary hover:text-text-secondary bg-transparent border-none cursor-pointer transition-colors"
                >
                  {t('onboarding.skip')}
                </button>
              )}
              <motion.button
                onClick={onNext}
                disabled={!canNext}
                whileTap={canNext ? { scale: 0.97 } : undefined}
                transition={spring.snappy}
                className="jelly-btn-accent px-5 py-2 text-[13px] font-medium border-none disabled:opacity-40 disabled:cursor-not-allowed"
              >
                {nextLabel ?? t('onboarding.next')}
              </motion.button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
