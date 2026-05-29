import { useState, useMemo } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useTranslation } from 'react-i18next'
import { Search, Copy, Check, Trash2 } from 'lucide-react'
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
    <div className="w-full h-full flex flex-col text-text-primary bg-bg-primary">
      {/* ── Header ── */}
      <div className="flex items-center gap-3 px-5 py-3 border-b border-border bg-bg-secondary shrink-0">
        <span
          className="text-[11px] text-text-tertiary uppercase tracking-widest shrink-0"
          style={{ fontFamily: 'var(--font-display)' }}
        >
          {t('history.title')}
        </span>
        <div className="flex-1 flex items-center gap-2 px-3 py-1.5 rounded-md bg-bg-primary border border-border focus-within:border-border-strong transition-colors">
          <Search size={12} strokeWidth={2} className="text-text-tertiary shrink-0" />
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t('history.searchPlaceholder')}
            className="flex-1 bg-transparent border-none text-[12px] text-text-primary outline-none placeholder:text-text-tertiary min-w-0"
          />
          {search && (
            <button
              onClick={() => setSearch('')}
              className="shrink-0 text-text-tertiary hover:text-text-primary bg-transparent border-none cursor-pointer text-[11px] leading-none"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* ── Log ── */}
      <div className="flex-1 overflow-y-auto">
        {filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-2 text-center px-8">
            <Search size={24} strokeWidth={1} className="text-text-tertiary mb-1" />
            <p className="text-[13px] text-text-secondary">
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
                <div className="sticky top-0 z-10 px-5 py-2 border-b border-border bg-bg-secondary/95 backdrop-blur-sm flex items-center gap-3">
                  <div className="h-px flex-1 bg-border" />
                  <span
                    className="text-[10px] text-text-tertiary uppercase tracking-widest shrink-0"
                    style={{ fontFamily: 'var(--font-display)' }}
                  >
                    {label}
                  </span>
                  <div className="h-px flex-1 bg-border" />
                </div>

                {/* Entries */}
                {entries.map((entry) => {
                  const time = entry.created_at.split('T')[1]?.slice(0, 5) || ''
                  const isCopied = copiedId === entry.id
                  return (
                    <div
                      key={entry.id}
                      className="group flex items-stretch border-b border-border transition-colors duration-150 ease-out hover:bg-bg-secondary"
                    >
                      {/* Left meta column */}
                      <div
                        className="shrink-0 flex flex-col justify-center gap-1 px-3.5 py-1 border-r border-border"
                        style={{ width: '120px' }}
                      >
                        <span
                          className="text-[12px] text-text-primary tabular-nums leading-none"
                          style={{ fontFamily: 'var(--font-display)' }}
                        >
                          {time}
                        </span>
                        {entry.app_name && (
                          <span
                            className="text-[10px] text-text-tertiary truncate leading-none mt-0.5"
                            style={{ fontFamily: 'var(--font-display)' }}
                            title={entry.app_name}
                          >
                            {entry.app_name}
                          </span>
                        )}
                      </div>

                      {/* Text + action column */}
                      <div className="flex-1 min-w-0 flex items-center gap-3 px-5 py-3.5">
                        <p className="flex-1 text-[13px] text-text-primary leading-relaxed min-w-0">
                          {entry.polished_text}
                        </p>

                        <motion.button
                          onClick={() => handleCopy(entry.id, entry.polished_text)}
                          whileTap={{ scale: 0.88 }}
                          transition={spring.snappy}
                          className={`shrink-0 w-7 h-7 flex items-center justify-center rounded-md border cursor-pointer transition-all ${
                            isCopied
                              ? 'opacity-100 bg-green-50 border-green-200 text-green-600 dark:bg-green-900/20 dark:border-green-800 dark:text-green-400'
                              : 'opacity-0 group-hover:opacity-100 bg-transparent border-border text-text-tertiary hover:text-text-primary hover:border-border-strong hover:bg-bg-tertiary'
                          }`}
                          aria-label={`Copy: ${entry.polished_text.slice(0, 30)}`}
                        >
                          {isCopied ? (
                            <Check size={11} strokeWidth={2.5} />
                          ) : (
                            <Copy size={11} strokeWidth={1.75} />
                          )}
                        </motion.button>
                      </div>
                    </div>
                  )
                })}
              </div>
            ))}
          </AnimatePresence>
        )}
      </div>

      {/* ── Footer ── */}
      {history.length > 0 && (
        <div className="shrink-0 border-t border-border bg-bg-secondary px-5 py-2.5 flex items-center justify-between">
          <span className="text-[11px] text-text-tertiary" style={{ fontFamily: 'var(--font-display)' }}>
            {history.length} {t('history.title').toLowerCase()}
          </span>
          <button
            onClick={handleClear}
            className="flex items-center gap-1.5 text-[11px] text-text-tertiary hover:text-error bg-transparent border-none cursor-pointer transition-colors"
            style={{ fontFamily: 'var(--font-display)' }}
          >
            <Trash2 size={11} strokeWidth={1.75} />
            {t('history.clearAll')}
          </button>
        </div>
      )}
    </div>
  )
}
