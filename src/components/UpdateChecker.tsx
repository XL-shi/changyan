import { useEffect, useState } from 'react'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { AnimatePresence, motion } from 'framer-motion'
import { Download, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { spring } from '../lib/animations'

export function UpdateChecker() {
  const { t } = useTranslation()
  const [update, setUpdate] = useState<Update | null>(null)
  const [downloading, setDownloading] = useState(false)
  const [dismissed, setDismissed] = useState(false)

  useEffect(() => {
    check()
      .then((u) => {
        if (u?.available) setUpdate(u)
      })
      .catch(() => {})
  }, [])

  async function handleUpdate() {
    if (!update) return
    setDownloading(true)
    try {
      await update.downloadAndInstall()
      await relaunch()
    } catch {
      setDownloading(false)
    }
  }

  return (
    <AnimatePresence>
      {update && !dismissed && (
        <motion.div
          className="fixed bottom-4 right-4 z-50 flex items-start gap-3 rounded-xl border border-border bg-surface shadow-lg px-4 py-3 w-72"
          initial={{ opacity: 0, y: 16, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 16, scale: 0.96 }}
          transition={spring}
        >
          <Download className="text-accent mt-0.5 shrink-0" size={16} />
          <div className="flex-1 min-w-0">
            <p className="text-[13px] font-medium text-text-primary">
              {t('app.update.available')}
            </p>
            <p className="text-[12px] text-text-secondary mt-0.5">
              {t('app.update.ready', { version: update.version })}
            </p>
            <div className="flex gap-2 mt-2">
              <button
                onClick={handleUpdate}
                disabled={downloading}
                className="jelly-btn-accent text-[12px] px-3 py-1 border-none disabled:opacity-50"
              >
                {downloading ? t('app.update.installing') : t('app.update.updateNow')}
              </button>
              <button
                onClick={() => setDismissed(true)}
                className="text-[12px] text-text-tertiary hover:text-text-secondary px-2 py-1"
              >
                {t('app.update.later')}
              </button>
            </div>
          </div>
          <button onClick={() => setDismissed(true)} className="text-text-tertiary hover:text-text-secondary shrink-0">
            <X size={14} />
          </button>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
