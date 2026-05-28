import { useTranslation } from 'react-i18next'
import i18n from '../../i18n'
import { useAppStore } from '../../stores/appStore'
import type { UiLanguage } from '../../stores/appStore'
import { LANGUAGES } from '../../lib/constants'

const UI_LANGUAGES: { value: UiLanguage; label: string }[] = [
  { value: 'en', label: 'English' },
  { value: 'zh', label: '中文' },
]

export function WelcomeStep() {
  const { t } = useTranslation()
  const config = useAppStore((s) => s.config)
  const updateConfig = useAppStore((s) => s.updateConfig)

  const handleUiLanguage = (lang: UiLanguage) => {
    updateConfig({ ui_language: lang })
    i18n.changeLanguage(lang)
  }

  return (
    <div className="space-y-7">
      {/* UI Language */}
      <div>
        <p className="cy-label mb-3">{t('settings.uiLanguage')}</p>
        <div className="flex gap-2">
          {UI_LANGUAGES.map((lang) => (
            <button
              key={lang.value}
              onClick={() => handleUiLanguage(lang.value)}
              className={`flex-1 px-4 py-3 text-[13px] border cursor-pointer transition-colors rounded-[4px] ${
                config.ui_language === lang.value
                  ? 'border-[var(--color-accent)] text-text-primary font-medium bg-[var(--color-accent-light)]'
                  : 'border-border text-text-secondary bg-bg-secondary hover:border-border-strong hover:text-text-primary'
              }`}
            >
              {lang.label}
            </button>
          ))}
        </div>
      </div>

      {/* Recognition Language */}
      <div>
        <p className="cy-label mb-3">{t('onboarding.recognitionLanguage')}</p>
        <div className="grid grid-cols-2 gap-2">
          {LANGUAGES.map((lang) => (
            <button
              key={lang.value}
              onClick={() => updateConfig({ stt_language: lang.value })}
              className={`px-4 py-2.5 text-[13px] border cursor-pointer transition-colors text-left rounded-[4px] ${
                config.stt_language === lang.value
                  ? 'border-[var(--color-accent)] text-text-primary font-medium bg-[var(--color-accent-light)]'
                  : 'border-border text-text-secondary bg-bg-secondary hover:border-border-strong hover:text-text-primary'
              }`}
            >
              {lang.label}
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
