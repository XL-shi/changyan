import { useState, useEffect, useRef, useCallback } from 'react'
import { AnimatePresence } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { SettingsSidebar, type PaneId } from './SettingsSidebar'
import { GeneralPane } from './GeneralPane'
import { SttPane } from './SttPane'
import { LlmPane } from './LlmPane'
import { DictionaryPane } from './DictionaryPane'
// import { ScenesPane } from './ScenesPane'
// import { AboutPane } from './AboutPane'
import { DirtyBar, useDirtyConfig } from './shared/DirtyBar'

const paneTitleKeys: Record<PaneId, string> = {
  general: 'settings.general',
  stt: 'settings.speechRecognition',
  llm: 'settings.aiPolish',
  dictionary: 'settings.dictionary',
  // scenes: 'settings.scenes',
  // about: 'settings.about',
}

export function Settings() {
  const [activePane, setActivePane] = useState<PaneId>('general')
  const scrollRef = useRef<HTMLDivElement>(null)
  const config = useAppStore((s) => s.config)
  const setSavedConfig = useAppStore((s) => s.setSavedConfig)
  const isDirty = useDirtyConfig()
  const { t } = useTranslation()

  const handleSelectPane = useCallback((pane: PaneId) => {
    setActivePane(pane)
    scrollRef.current?.scrollTo({ top: 0 })
  }, [])

  // Snapshot config when settings opens
  useEffect(() => {
    setSavedConfig(config)
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="w-full h-full bg-bg-primary text-text-primary flex flex-col">
      <div className="flex-1 flex min-h-0">
        {/* Sidebar */}
        <SettingsSidebar activePane={activePane} onSelect={handleSelectPane} />

        {/* Content */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Title bar */}
          <div className="flex items-center px-8 py-4 border-b border-border bg-bg-secondary">
            <h2 className="text-[13px] font-medium text-text-primary">{t(paneTitleKeys[activePane])}</h2>
          </div>

          {/* Pane content */}
          <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-2 bg-bg-primary">
            <div className="w-full">
              {activePane === 'general' && <GeneralPane />}
              {activePane === 'stt' && <SttPane />}
              {activePane === 'llm' && <LlmPane />}
              {activePane === 'dictionary' && <DictionaryPane />}
              {/* {activePane === 'scenes' && <ScenesPane />} */}
              {/* {activePane === 'about' && <AboutPane />} */}
            </div>
          </div>
        </div>
      </div>

      {/* Dirty bar */}
      <AnimatePresence>{isDirty && <DirtyBar />}</AnimatePresence>
    </div>
  )
}
