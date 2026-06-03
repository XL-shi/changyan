import { ArrowRight, Crown, Mic, Zap } from 'lucide-react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { spring } from '../../lib/animations'
import { useAppStore } from '../../stores/appStore'
import { useAuthStore } from '../../stores/authStore'
import { useRoute } from '../../lib/router'

export function HomePage() {
  const config = useAppStore((s) => s.config)
  const history = useAppStore((s) => s.history)
  const { navigate } = useRoute()
  const { user, plan, sttSecondsUsed, sttSecondsLimit, llmTokensUsed, llmTokensLimit } =
    useAuthStore()
  const { t } = useTranslation()
  const isPro = plan === 'pro'

  const now = new Date()
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
  const todayCount = history.filter((h) => h.created_at.startsWith(today)).length

  return (
    <div className="h-full flex flex-col overflow-y-auto bg-bg-primary">
      {/* ── Hotkey bar ── */}
      <div
        className="flex items-center justify-between px-6 py-3 border-b border-border bg-bg-secondary"
        style={{ minHeight: '44px' }}
      >
        <div className="flex items-center gap-2">
          <Mic size={12} strokeWidth={1.75} className="text-text-tertiary" />
          <p className="text-[12px] text-text-tertiary">{t('home.recordHint')}</p>
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {config.hotkey.split('+').map((k) => (
            <kbd
              key={k}
              className="inline-flex items-center justify-center px-2.5 py-1 min-w-[32px] h-[26px] rounded-[6px] text-[11px] font-mono font-semibold leading-none select-none
                text-text-primary bg-gradient-to-b from-bg-secondary to-bg-tertiary border border-border
                shadow-[0_2px_0_0_rgba(0,0,0,0.15),0_1px_2px_rgba(0,0,0,0.08)]"
            >
              {k.trim()}
            </kbd>
          ))}
        </div>
      </div>

      <div className="flex-1 px-6 py-5 flex flex-col gap-4 min-h-0">
        {/* ── Stat cards ── */}
        <div className="grid grid-cols-2 gap-3">
          <StatCard value={history.length} label={t('home.totalRecordings')} />
          <StatCard value={todayCount} label={t('home.today')} highlight />
        </div>

        {/* ── Config panel ── */}
        <div className="bg-bg-secondary border border-border rounded-lg overflow-hidden shadow-sm">
          <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border bg-bg-primary">
            <Zap size={11} strokeWidth={2} className="text-text-tertiary" />
            <span className="cy-label">{t('home.currentConfig')}</span>
          </div>
          <div>
            <ConfigRow label={t('home.sttProvider')} value={config.stt_provider} />
            <ConfigRow label={t('home.llmProvider')} value={config.llm_provider} />
            <ConfigRow
              label={t('home.aiPolish')}
              value={config.polish_enabled ? t('home.enabled') : t('home.disabled')}
              isStatus
              statusOk={config.polish_enabled}
            />
            <ConfigRow label={t('home.outputMode')} value={config.output_mode} last />
          </div>
        </div>

        {/* ── Plan / Quota — cloud users only ── */}
        {user && (
          <div className="bg-bg-secondary border border-border rounded-lg overflow-hidden shadow-sm">
            <div className="flex items-center justify-between px-4 py-2.5 border-b border-border bg-bg-primary">
              <div className="flex items-center gap-2">
                {isPro && <Crown size={11} className="text-amber-500" />}
                <span className="cy-label">{isPro ? t('home.proPlan') : t('home.freePlan')}</span>
              </div>
              {!isPro && (
                <button
                  onClick={() => navigate('upgrade')}
                  className="text-[11px] text-text-tertiary hover:text-text-primary bg-transparent border-none cursor-pointer transition-colors"
                  style={{ fontFamily: 'var(--font-display)', letterSpacing: '0.04em' }}
                >
                  {t('home.upgradeToPro')} →
                </button>
              )}
            </div>
            {sttSecondsLimit > 0 && (
              <div className="px-4 py-3 space-y-3">
                <QuotaRow
                  label={t('upgrade.stt')}
                  used={sttSecondsUsed}
                  limit={sttSecondsLimit}
                  unit="h"
                  divisor={3600}
                />
                <QuotaRow
                  label={t('upgrade.llm')}
                  used={llmTokensUsed}
                  limit={llmTokensLimit}
                  unit="k"
                  divisor={1000}
                />
              </div>
            )}
          </div>
        )}

        {/* ── Quick actions ── */}
        <div className="mt-auto grid grid-cols-2 gap-3 pt-1">
          <ActionButton label={t('nav.settings')} onClick={() => navigate('settings')} />
          <ActionButton label={t('nav.history')} onClick={() => navigate('history')} />
        </div>
      </div>
    </div>
  )
}

/* ── Sub-components ── */

function StatCard({
  value,
  label,
  highlight,
}: {
  value: number
  label: string
  highlight?: boolean
}) {
  return (
    <div
      className={`rounded-lg border px-5 py-4 shadow-sm ${
        highlight
          ? 'bg-bg-secondary border-border-strong'
          : 'bg-bg-secondary border-border'
      }`}
      style={highlight ? { borderTopWidth: '2px', borderTopColor: 'var(--color-text-primary)' } : undefined}
    >
      <p className="cy-stat text-[42px] leading-none text-text-primary">{value}</p>
      <p
        className="mt-2 text-[10px] uppercase tracking-widest text-text-tertiary"
        style={{ fontFamily: 'var(--font-display)' }}
      >
        {label}
      </p>
    </div>
  )
}

function ConfigRow({
  label,
  value,
  isStatus,
  statusOk,
  last,
}: {
  label: string
  value: string
  isStatus?: boolean
  statusOk?: boolean
  last?: boolean
}) {
  return (
    <div
      className={`flex items-center justify-between px-4 py-2.5 ${last ? '' : 'border-b border-border'}`}
    >
      <span className="text-[12px] text-text-secondary">{label}</span>
      {isStatus ? (
        <span
          className={`text-[11px] px-2 py-0.5 rounded-full border ${
            statusOk
              ? 'text-green-700 bg-green-50 border-green-200 dark:text-green-400 dark:bg-green-900/20 dark:border-green-800'
              : 'text-text-tertiary bg-bg-tertiary border-border'
          }`}
          style={{ fontFamily: 'var(--font-display)' }}
        >
          {value}
        </span>
      ) : (
        <span
          className="text-[12px] text-text-primary max-w-[160px] truncate"
          style={{ fontFamily: 'var(--font-display)' }}
        >
          {value}
        </span>
      )}
    </div>
  )
}

function ActionButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <motion.button
      onClick={onClick}
      whileHover={{ y: -1 }}
      whileTap={{ scale: 0.97 }}
      transition={spring.snappy}
      className="flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg bg-bg-secondary border border-border text-[12px] text-text-secondary hover:text-text-primary hover:border-border-strong cursor-pointer transition-all shadow-sm hover:shadow-md"
      style={{ fontFamily: 'var(--font-sans)' }}
    >
      <ArrowRight size={12} strokeWidth={1.75} />
      {label}
    </motion.button>
  )
}

function QuotaRow({
  label,
  used,
  limit,
  unit,
  divisor,
}: {
  label: string
  used: number
  limit: number
  unit: string
  divisor: number
}) {
  const pct = limit > 0 ? Math.min((used / limit) * 100, 100) : 0
  const usedDisplay = (used / divisor).toFixed(1)
  const limitDisplay = (limit / divisor).toFixed(1)

  return (
    <div>
      <div className="flex justify-between items-baseline mb-1.5">
        <span className="text-[12px] text-text-secondary">{label}</span>
        <span
          className="text-[11px] text-text-tertiary"
          style={{ fontFamily: 'var(--font-display)' }}
        >
          {usedDisplay}/{limitDisplay}
          {unit}
        </span>
      </div>
      <div className="h-[3px] bg-bg-tertiary rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full transition-all ${pct > 90 ? 'bg-error' : 'bg-text-secondary'}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  )
}
