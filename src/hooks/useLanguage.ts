import { useEffect } from 'react'
import i18n from '../i18n'
import { useAppStore } from '../stores/appStore'

export function useLanguage() {
  const uiLanguage = useAppStore((s) => s.config.ui_language)

  useEffect(() => {
    i18n.changeLanguage(uiLanguage ?? 'en')
  }, [uiLanguage])
}
