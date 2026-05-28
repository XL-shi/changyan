import { ArrowRight, Crown } from 'lucide-react'
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
    <div className="h-full flex flex-col overflow-y-auto">
      {/* ── Hotkey hint ── */}
      <div className="flex items-center justify-between px-8 py-4 border-b border-border">
        <p className="text-[13px] text-text-secondary">{t('home.recordHint')}</p>
        <kbd
          className="shrink-0 ml-4 px-2 py-0.5 rounded text-[11px] text-text-primary border border-border-strong"
          style={{ fontFamily: 'var(--font-display)' }}
        >
          {config.hotkey}
        </kbd>
      </div>

      {/* ── Hero stats ── */}
      <div className="px-8 py-8 border-b border-border">
        <div className="grid grid-cols-2">
          <div className="pr-8 border-r border-border">
            <p className="cy-stat text-[52px] text-text-primary">{history.length}</p>
            <p className="cy-label mt-2">{t('home.totalRecordings')}</p>
          </div>
          <div className="pl-8">
            <p className="cy-stat text-[52px] text-text-primary">{todayCount}</p>
            <p className="cy-label mt-2">{t('home.today')}</p>
          </div>
        </div>
      </div>

      {/* ── Configuration ── */}
      <div className="px-8 py-6 border-b border-border">
        <p className="cy-label mb-4">{t('home.currentConfig')}</p>
        <div className="grid grid-cols-2 gap-x-6 gap-y-4">
          <ConfigItem label={t('home.sttProvider')} value={config.stt_provider} />
          <ConfigItem label={t('home.llmProvider')} value={config.llm_provider} />
          <ConfigItem
            label={t('home.aiPolish')}
            value={config.polish_enabled ? t('home.enabled') : t('home.disabled')}
          />
          <ConfigItem label={t('home.outputMode')} value={config.output_mode} />
        </div>
      </div>

      {/* ── Plan / Quota — cloud users only ── */}
      {user && (
        <div className="px-8 py-6 border-b border-border">
          <div className="flex items-center gap-2 mb-4">
            {isPro && <Crown size={11} className="text-amber-500" />}
            <p className="cy-label">{isPro ? t('home.proPlan') : t('home.freePlan')}</p>
            {!isPro && (
              <button
                onClick={() => navigate('upgrade')}
                className="ml-auto text-[11px] text-text-secondary hover:text-text-primary bg-transparent border-none cursor-pointer transition-colors underline"
                style={{ fontFamily: 'var(--font-display)' }}
              >
                {t('home.upgradeToPro')}
              </button>
            )}
          </div>
          {sttSecondsLimit > 0 && (
            <div className="space-y-4">
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
      <div className="px-8 py-6 flex gap-8">
        <NavLink label={t('nav.settings')} onClick={() => navigate('settings')} />
        <NavLink label={t('nav.history')} onClick={() => navigate('history')} />
      </div>
    </div>
  )
}

/* ── Sub-components ── */

function ConfigItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <p className="cy-label mb-1">{label}</p>
      <p className="cy-config-value">{value}</p>
    </div>
  )
}

function NavLink({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <motion.button
      onClick={onClick}
      whileHover={{ x: 2 }}
      whileTap={{ scale: 0.97 }}
      transition={spring.snappy}
      className="flex items-center gap-1.5 text-[13px] text-text-secondary hover:text-text-primary bg-transparent border-none cursor-pointer transition-colors"
    >
      <ArrowRight size={13} strokeWidth={1.75} />
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
          {usedDisplay}/{limitDisplay}{unit}
        </span>
      </div>
      <div className="h-px bg-border overflow-hidden rounded-full">
        <div
          className={`h-full transition-all ${pct > 90 ? 'bg-error' : 'bg-text-secondary'}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  )
}
