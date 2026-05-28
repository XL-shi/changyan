import { motion } from 'framer-motion'
import { Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'

interface DoneStepProps {
  isFinalizing?: boolean
  modelDownloadProgress?: number
  modelDownloadError?: string | null
  showModelDownloadStatus?: boolean
}

export function DoneStep({
  isFinalizing = false,
  modelDownloadProgress = 0,
  modelDownloadError = null,
  showModelDownloadStatus = false,
}: DoneStepProps) {
  const { t } = useTranslation()
  const config = useAppStore((s) => s.config)

  return (
    <div className="space-y-6">
      {/* Status */}
      {showModelDownloadStatus && (isFinalizing || modelDownloadError) && (
        <div className="flex items-center gap-3 px-4 py-3 border border-border rounded-[4px] bg-bg-secondary">
          {isFinalizing ? (
            <>
              <motion.div animate={{ rotate: 360 }} transition={{ repeat: Infinity, duration: 1, ease: 'linear' }}>
                <Loader2 size={13} className="text-text-tertiary" />
              </motion.div>
              <span className="text-[12px] text-text-secondary" style={{ fontFamily: 'var(--font-display)' }}>
                {t('settings.downloadingModel')} {Math.round(modelDownloadProgress)}%
              </span>
            </>
          ) : (
            <p className="text-[12px] text-error">{modelDownloadError}</p>
          )}
        </div>
      )}

      {/* Shortcuts */}
      <div>
        <p className="cy-label mb-3">{t('onboarding.allSet')}</p>
        <div className="space-y-2">
          <ShortcutRow
            title={t('settings.voiceInputHotkey')}
            desc={t('settings.voiceInputHotkeyDesc')}
            hotkey={config.hotkey}
          />
          <ShortcutRow
            title={t('settings.translateHotkey')}
            desc={t('settings.translateHotkeyDesc')}
            hotkey={config.translate_hotkey}
          />
        </div>
      </div>
    </div>
  )
}

function ShortcutRow({
  title,
  desc,
  hotkey,
}: {
  title: string
  desc: string
  hotkey?: string
}) {
  const chips = parseHotkeyChips(hotkey)

  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3 border border-border rounded-[4px] bg-bg-secondary">
      <div className="min-w-0">
        <p className="text-[13px] font-medium text-text-primary">{title}</p>
        <p className="text-[11px] text-text-tertiary mt-0.5">{desc}</p>
      </div>
      <div className="flex items-center gap-1 shrink-0">
        {chips.length ? (
          chips.map((chip) => (
            <kbd
              key={`${title}-${chip}`}
              className="px-2 py-0.5 text-[11px] border border-border rounded-[3px] text-text-primary bg-bg-tertiary"
              style={{ fontFamily: 'var(--font-display)' }}
            >
              {chip}
            </kbd>
          ))
        ) : (
          <span
            className="px-2 py-0.5 text-[11px] border border-border rounded-[3px] text-text-tertiary bg-bg-tertiary"
            style={{ fontFamily: 'var(--font-display)' }}
          >
            —
          </span>
        )}
      </div>
    </div>
  )
}

function parseHotkeyChips(hotkey?: string): string[] {
  if (!hotkey) return []
  return hotkey.split('+').map((part) => normalizeHotkeyPart(part.trim()))
}

function normalizeHotkeyPart(part: string): string {
  if (!part) return part
  if (part.toLowerCase() === 'fn') return 'Fn'
  return part.length === 1 ? part.toUpperCase() : part[0].toUpperCase() + part.slice(1)
}
