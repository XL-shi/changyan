import { useAppStore } from '../../stores/appStore'
import { LANGUAGES } from '../../lib/constants'

export function WelcomeStep() {
  const config = useAppStore((s) => s.config)
  const updateConfig = useAppStore((s) => s.updateConfig)

  return (
    <div className="space-y-6">
      <div className="text-center py-4">
        <div className="text-[40px] mb-2">🎙</div>
        <p className="text-[15px] text-text-secondary leading-relaxed">
          Speak to write — turn your voice into polished text in real time
        </p>
      </div>

      <div>
        <p className="text-[13px] font-medium text-text-secondary mb-3">Recognition Language</p>
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
