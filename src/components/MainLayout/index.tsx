import { Home, Settings, History } from 'lucide-react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { spring } from '../../lib/animations'
import { useRoute, type Route } from '../../lib/router'

const APP_VERSION = '0.3.3'

const NAV_ITEMS: { id: Route; num: string; labelKey: string; icon: typeof Home }[] = [
  { id: 'home', num: '01', labelKey: 'nav.home', icon: Home },
  { id: 'settings', num: '02', labelKey: 'nav.settings', icon: Settings },
  { id: 'history', num: '03', labelKey: 'nav.history', icon: History },
]

interface Props {
  children: React.ReactNode
}

export function MainLayout({ children }: Props) {
  const { route, navigate } = useRoute()
  const { t } = useTranslation()

  return (
    <div className="w-full h-full flex bg-bg-primary text-text-primary">
      {/* ── Sidebar ── */}
      <aside className="w-[200px] flex flex-col bg-bg-secondary border-r border-border shrink-0">
        {/* Brand */}
        <div className="px-6 pt-7 pb-5" data-tauri-drag-region>
          <h1 className="cy-mark text-[22px] text-text-primary leading-tight">{t('app.name')}</h1>
          <p
            className="mt-1.5 text-[10px] text-text-tertiary tracking-[0.1em] uppercase"
            style={{ fontFamily: 'var(--font-display)' }}
          >
            {t('app.tagline')}
          </p>
        </div>

        {/* Divider */}
        <div className="cy-rule" />

        {/* Nav */}
        <nav className="flex-1 py-2" aria-label="Main navigation">
          {NAV_ITEMS.map(({ id, num, labelKey, icon: Icon }) => {
            const active = route === id
            const label = t(labelKey)
            return (
              <motion.button
                key={id}
                onClick={() => navigate(id)}
                whileTap={{ scale: 0.98 }}
                transition={spring.snappy}
                aria-label={label}
                aria-current={active ? 'page' : undefined}
                className="relative w-full bg-transparent border-none cursor-pointer text-left"
              >
                {active && (
                  <motion.div
                    layoutId="nav-active-bg"
                    className="absolute inset-x-2 inset-y-0.5 rounded-md bg-bg-primary pointer-events-none"
                    transition={spring.snappy}
                  />
                )}

                <div
                  className={`relative flex items-center gap-3 px-4 py-2.5 transition-colors ${
                    active ? 'text-text-primary' : 'text-text-secondary hover:text-text-primary'
                  }`}
                >
                  <span
                    className="text-[10px] w-4 shrink-0 text-text-tertiary"
                    style={{ fontFamily: 'var(--font-display)' }}
                  >
                    {num}
                  </span>
                  <Icon size={13} strokeWidth={active ? 2 : 1.5} />
                  <span className={`text-[13px] ${active ? 'font-semibold' : ''}`}>{label}</span>
                </div>
              </motion.button>
            )
          })}
        </nav>

        {/* Version */}
        <div className="px-6 pb-5">
          <span
            className="text-[10px] text-text-tertiary"
            style={{ fontFamily: 'var(--font-display)' }}
          >
            v{APP_VERSION}
          </span>
        </div>
      </aside>

      {/* ── Content ── */}
      <main className="flex-1 min-w-0 overflow-hidden bg-bg-primary">{children}</main>
    </div>
  )
}
