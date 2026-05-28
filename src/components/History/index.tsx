import { useState, useMemo } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useTranslation } from 'react-i18next'
import { Search, Copy } from 'lucide-react'
import { spring } from '../../lib/animations'
import { useAppStore } from '../../stores/appStore'
import { clearHistory } from '../../lib/tauri'
import { toast } from '../Toast'

export function History() {
  const history = useAppStore((s) => s.history)
  const setHistory = useAppStore((s) => s.setHistory)
  const { t } = useTranslation()
  const [search, setSearch] = useState('')
  const [copiedId, setCopiedId] = useState<number | null>(null)

  const filtered = useMemo(
    () =>
      search
        ? history.filter(
            (h) =>
              h.polished_text.includes(search) ||
              h.raw_text.includes(search) ||
              h.app_name.includes(search),
          )
        : history,
    [history, search],
  )

  const handleCopy = (id: number, text: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => {
        setCopiedId(id)
        setTimeout(() => setCopiedId(null), 1500)
      })
      .catch(() => {
        toast.error(t('history.failedToCopy'))
      })
  }

  const handleClear = async () => {
    try {
      const confirmed = await confirm(t('history.clearConfirm'), { kind: 'warning' })
      if (!confirmed) return
      await clearHistory()
      setHistory([])
    } catch (e) {
      console.error('Failed to clear history:', e)
      toast.error(t('history.failedToClear'))
    }
  }

  const grouped = useMemo(() => {
    const map = new Map<string, typeof filtered>()
    for (const entry of filtered) {
      const date = entry.created_at.split('T')[0] || entry.created_at.split(' ')[0]
      const today = new Date().toISOString().split('T')[0]
      const yesterday = new Date(Date.now() - 86400000).toISOString().split('T')[0]
      const label =
        date === today ? t('history.today') : date === yesterday ? t('history.yesterday') : date
      if (!map.has(label)) map.set(label, [])
      map.get(label)!.push(entry)
    }
    return map
  }, [filtered, t])

  return (
    <div className="w-full h-full flex flex-col text-text-primary">
      {/* ── Header ── */}
      <div className="flex items-center gap-4 px-8 py-4 border-b border-border bg-bg-secondary">
        <h2
          className="text-[11px] text-text-tertiary uppercase tracking-widest shrink-0"
          style={{ fontFamily: 'var(--font-display)' }}
        >
          {t('history.title')}
        </h2>
        <div className="relative flex-1">
          <Search
            size={12}
            className="absolute left-0 top-1/2 -translate-y-1/2 text-text-tertiary"
            strokeWidth={2}
          />
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t('history.searchPlaceholder')}
            className="w-full pl-5 pr-0 py-0 bg-transparent border-none text-[13px] text-text-primary outline-none placeholder:text-text-tertiary"
            style={{ transform: 'none' }}
          />
        </div>
      </div>

      {/* ── Log ── */}
      <div className="flex-1 overflow-y-auto bg-bg-primary">
        {filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-1">
            <p className="cy-label">
              {search ? t('history.noResults') : t('history.noHistory')}
            </p>
            {!search && (
              <p className="text-[11px] text-text-tertiary">{t('history.noHistoryHint')}</p>
            )}
          </div>
        ) : (
          <AnimatePresence>
            {Array.from(grouped.entries()).map(([label, entries]) => (
              <div key={label}>
                {/* Date group header */}
                <div className="px-8 py-2.5 border-b border-border bg-bg-secondary">
                  <span className="cy-label">{label}</span>
                </div>

                {/* Entries */}
                {entries.map((entry) => {
                  const time = entry.created_at.split('T')[1]?.slice(0, 5) || ''
                  return (
                    <motion.div
                      key={entry.id}
                      whileHover={{ backgroundColor: 'var(--color-bg-secondary)' }}
                      transition={spring.snappy}
                      className="group flex items-start gap-0 border-b border-border"
                    >
                      {/* Time + app column */}
                      <div
                        className="shrink-0 px-8 py-3.5 border-r border-border"
                        style={{ width: '140px' }}
                      >
                        <p
                          className="text-[11px] text-text-tertiary"
                          style={{ fontFamily: 'var(--font-display)' }}
                        >
                          {time}
                        </p>
                        <p
                          className="text-[11px] text-text-secondary mt-0.5 truncate"
                          style={{ fontFamily: 'var(--font-display)' }}
                        >
                          {entry.app_name}
                        </p>
                      </div>

                      {/* Text column */}
                      <div className="flex-1 min-w-0 px-5 py-3.5 flex items-start gap-3">
                        <p className="flex-1 text-[13px] text-text-primary leading-relaxed min-w-0">
                          {entry.polished_text}
                        </p>

                        {/* Copy / Copied */}
                        {copiedId === entry.id ? (
                          <span
                            className="shrink-0 text-[10px] text-success self-center"
                            style={{ fontFamily: 'var(--font-display)' }}
                          >
                            {t('history.copied')}
                          </span>
                        ) : (
                          <motion.button
                            onClick={() => handleCopy(entry.id, entry.polished_text)}
                            whileTap={{ scale: 0.9 }}
                            transition={spring.snappy}
                            className="shrink-0 opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-bg-tertiary transition-all bg-transparent border-none cursor-pointer text-text-tertiary hover:text-text-primary self-center"
                            aria-label={`Copy: ${entry.polished_text.slice(0, 30)}`}
                          >
                            <Copy size={12} strokeWidth={1.75} />
                          </motion.button>
                        )}
                      </div>
                    </motion.div>
                  )
                })}
              </div>
            ))}
          </AnimatePresence>
        )}
      </div>

      {/* ── Clear ── */}
      {history.length > 0 && (
        <div className="border-t border-border bg-bg-secondary px-8 py-3">
          <button
            onClick={handleClear}
            className="cy-label text-text-tertiary hover:text-error bg-transparent border-none cursor-pointer transition-colors"
          >
            {t('history.clearAll')}
          </button>
        </div>
      )}
    </div>
  )
}
