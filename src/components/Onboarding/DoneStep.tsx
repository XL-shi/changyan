import { motion } from 'framer-motion'
import { Check, Loader2 } from 'lucide-react'
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
    <div className="flex flex-col items-center gap-5 py-2">
      {/* Success animation */}
      <motion.div
        className="w-16 h-16 rounded-full bg-success/10 flex items-center justify-center"
        initial={{ scale: 0 }}
        animate={{ scale: 1 }}
        transition={{ type: 'spring', stiffness: 500, damping: 20 }}
      >
        <motion.div
          initial={{ opacity: 0, scale: 0 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.2, type: 'spring', stiffness: 500, damping: 20 }}
        >
          <Check size={28} className="text-success" />
        </motion.div>
      </motion.div>

      <div className="text-center">
        <h2 className="text-[17px] font-semibold text-text-primary">{t('onboarding.allSet')}</h2>
        {/* <p className="text-[13px] text-text-secondary mt-1">
          {t('onboarding.capsuleOnDesktop')}
        </p> */}
      </div>

      {showModelDownloadStatus && (isFinalizing || modelDownloadError) && (
        <div className="w-full rounded-[10px] bg-bg-secondary px-4 py-3 text-center">
          {isFinalizing ? (
            <div className="flex items-center justify-center gap-2 text-[12px] text-text-secondary">
              <Loader2 size={13} className="animate-spin" />
              <span>
                {t('settings.downloadingModel')} {Math.round(modelDownloadProgress)}%
              </span>
            </div>
          ) : (
            <p className="text-[12px] text-error">{modelDownloadError}</p>
          )}
        </div>
      )}

      {/* Usage tips */}
      <div className="w-full space-y-3">
        <ShortcutTip
          title={t('settings.voiceInputHotkey')}
          desc={t('settings.voiceInputHotkeyDesc')}
          hotkey={config.hotkey}
        />
        <ShortcutTip
          title={t('settings.translateHotkey')}
          desc={t('settings.translateHotkeyDesc')}
          hotkey={config.translate_hotkey}
        />
      </div>
    </div>
  )
}

function ShortcutTip({
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
    <div className="flex items-center justify-between gap-3 px-4 py-3 bg-bg-secondary rounded-[10px]">
      <div className="min-w-0">
        <p className="text-[13px] font-medium text-text-primary">{title}</p>
        <p className="text-[11px] text-text-tertiary">{desc}</p>
      </div>
      <div className="flex items-center gap-1.5 shrink-0">
        {chips.length ? (
          chips.map((chip) => (
            <span
              key={`${title}-${chip}`}
              className="px-2.5 py-1 rounded-[10px] text-[16px] font-mono bg-bg-tertiary border border-border text-text-primary leading-none"
            >
              {chip}
            </span>
          ))
        ) : (
          <span className="px-2.5 py-1 rounded-[10px] text-[12px] bg-bg-tertiary border border-border text-text-tertiary leading-none">
            -
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
