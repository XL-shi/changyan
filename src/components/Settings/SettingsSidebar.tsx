import { Settings, Mic, Sparkles, BookOpen } from 'lucide-react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { spring } from '../../lib/animations'

const PANES = [
  { id: 'general', labelKey: 'settings.general', icon: Settings },
  { id: 'stt', labelKey: 'settings.speechRecognition', icon: Mic },
  { id: 'llm', labelKey: 'settings.aiPolish', icon: Sparkles },
  { id: 'dictionary', labelKey: 'settings.dictionary', icon: BookOpen },
] as const

export type PaneId = (typeof PANES)[number]['id']

interface Props {
  activePane: PaneId
  onSelect: (id: PaneId) => void
}

export function SettingsSidebar({ activePane, onSelect }: Props) {
  const { t } = useTranslation()

  return (
    <div className="w-[180px] h-full bg-bg-secondary border-r border-border flex flex-col shrink-0">
      {/* Label */}
      <div className="px-6 py-4 border-b border-border">
        <span
          className="cy-label"
          style={{ fontFamily: 'var(--font-display)' }}
        >
          {t('settings.title')}
        </span>
      </div>

      {/* Nav */}
      <nav className="flex-1 pt-1" aria-label="Settings navigation">
        {PANES.map((pane) => {
          const Icon = pane.icon
          const isActive = activePane === pane.id
          return (
            <motion.button
              key={pane.id}
              onClick={() => onSelect(pane.id)}
              whileTap={{ scale: 0.98 }}
              transition={spring.snappy}
              className="relative w-full bg-transparent border-none cursor-pointer text-left"
            >
              {isActive && (
                <motion.div
                  layoutId="settings-nav-fence"
                  className="absolute inset-0 pointer-events-none"
                  style={{
                    borderTop: '1px solid var(--color-accent)',
                    borderBottom: '1px solid var(--color-accent)',
                  }}
                  transition={spring.snappy}
                />
              )}
              <div
                className={`flex items-center gap-2.5 px-6 py-3.5 transition-colors ${
                  isActive
                    ? 'text-text-primary'
                    : 'text-text-secondary hover:text-text-primary'
                }`}
              >
                <Icon size={13} strokeWidth={isActive ? 2 : 1.5} />
                <span className={`text-[13px] ${isActive ? 'font-medium' : ''}`}>
                  {t(pane.labelKey)}
                </span>
              </div>
            </motion.button>
          )
        })}
      </nav>
    </div>
  )
}
