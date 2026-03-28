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
    <div className="space-y-6">
      <div className="text-center py-4">
        <div className="text-[40px] mb-2">🎙</div>
        <p className="text-[15px] text-text-secondary leading-relaxed">
          {t('onboarding.speakToWrite')}
        </p>
      </div>

      <div>
        <p className="text-[13px] font-medium text-text-secondary mb-3">
          {t('settings.uiLanguage')}
        </p>
        <div className="flex gap-2">
          {UI_LANGUAGES.map((lang) => (
            <button
              key={lang.value}
              onClick={() => handleUiLanguage(lang.value)}
              className={`flex-1 px-4 py-3 rounded-[10px] text-[13px] border cursor-pointer transition-all ${
                config.ui_language === lang.value
                  ? 'bg-accent/10 border-accent text-accent font-medium'
                  : 'bg-bg-secondary border-border text-text-primary hover:border-text-tertiary'
              }`}
            >
              {lang.label}
            </button>
          ))}
        </div>
      </div>

      <div>
        <p className="text-[13px] font-medium text-text-secondary mb-3">
          {t('onboarding.recognitionLanguage')}
        </p>
        <div className="grid grid-cols-2 gap-2">
          {LANGUAGES.map((lang) => (
            <button
              key={lang.value}
              onClick={() => updateConfig({ stt_language: lang.value })}
              className={`px-4 py-3 rounded-[10px] text-[13px] border cursor-pointer transition-all ${
                config.stt_language === lang.value
                  ? 'bg-accent/10 border-accent text-accent font-medium'
                  : 'bg-bg-secondary border-border text-text-primary hover:border-text-tertiary'
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
