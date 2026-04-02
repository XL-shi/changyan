import { useEffect } from 'react'

export function useTheme() {
  useEffect(() => {
    const root = document.documentElement
    const mq = window.matchMedia('(prefers-color-scheme: dark)')

    function applyTheme(isDark: boolean) {
      root.classList.toggle('dark', isDark)
    }

    applyTheme(mq.matches)

    const handler = (e: MediaQueryListEvent) => {
      applyTheme(e.matches)
    }

    mq.addEventListener('change', handler)
    return () => mq.removeEventListener('change', handler)
  }, [])

  return 'system' as const
}
