import { useState, useCallback, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { Keyboard, Globe, X } from 'lucide-react'
import i18n from '../../i18n'
import { useAppStore } from '../../stores/appStore'
import type { UiLanguage } from '../../stores/appStore'
import {
  updateHotkey,
  pauseHotkey,
  resumeHotkey,
  updateTranslateHotkey,
  requestAccessibilityPermission,
  openAccessibilitySettings,
} from '../../lib/tauri'
import { TARGET_LANGUAGES } from '../../lib/constants'
import { Toggle } from './shared/Toggle'

// Keys that can be used as hotkeys without a modifier
const STANDALONE_KEYS = new Set([
  'Space',
  'Tab',
  'Enter',
  'Backspace',
  'Escape',
  'Delete',
  'Insert',
  'Home',
  'End',
  'PageUp',
  'PageDown',
  'Up',
  'Down',
  'Left',
  'Right',
  'F1',
  'F2',
  'F3',
  'F4',
  'F5',
  'F6',
  'F7',
  'F8',
  'F9',
  'F10',
  'F11',
  'F12',
])

/** Parse "Ctrl+Shift+/" → ["Ctrl", "Shift", "/"] for chip display */
function parseHotkeyChips(hotkey: string): string[] {
  if (!hotkey) return []
  return hotkey.split('+').map((k) => k.trim())
}

/** Display the hotkey as individual key chips */
function HotkeyChips({ hotkey, onClear }: { hotkey: string; onClear?: () => void }) {
  const chips = parseHotkeyChips(hotkey)
  if (!chips.length) return null
  return (
    <div className="flex items-center gap-1.5">
      {chips.map((chip, i) => (
        <span
          key={i}
          className="px-2 py-1 rounded-[8px] text-[12px] font-mono bg-bg-secondary border border-border text-text-primary leading-none"
        >
          {chip}
        </span>
      ))}
      {onClear && (
        <button
          onClick={(e) => {
            e.stopPropagation()
            onClear()
          }}
          className="ml-1 p-1 rounded-md text-text-tertiary hover:text-text-secondary transition-colors border-none bg-transparent cursor-pointer"
          title="Clear"
        >
          <X size={13} />
        </button>
      )}
    </div>
  )
}

interface HotkeyRecorderProps {
  value: string
  /** Called with the new hotkey string (or "" to clear) */
  onSave: (hotkey: string) => Promise<void>
  /** Whether this recorder clears on save rather than just updating */
  clearable?: boolean
}

function HotkeyRecorder({ value, onSave, clearable }: HotkeyRecorderProps) {
  const { t } = useTranslation()
  const [recording, setRecording] = useState(false)
  const [pending, setPending] = useState<string | null>(null)
  const [modifierHint, setModifierHint] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const autoConfirmTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  const confirmHotkey = useCallback(
    (hotkey: string) => {
      setRecording(false)
      setError(null)
      setModifierHint(null)
      onSave(hotkey)
        .then(() => setPending(null))
        .catch((e) => {
          setError(String(e))
          setPending(null)
          resumeHotkey().catch(() => {})
        })
    },
    [onSave],
  )

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      e.preventDefault()
      e.stopPropagation()

      // Fn modifier is not exposed by browsers; Fn combos are handled entirely
      // by the backend CGEventTap via hotkey:fn_combo_detected events.
      const parts: string[] = []
      if (e.ctrlKey) parts.push('Ctrl')
      if (e.altKey) parts.push('Alt')
      if (e.shiftKey) parts.push('Shift')
      if (e.metaKey) parts.push('Meta')

      if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
        setModifierHint(parts.length > 0 ? parts.join('+') + '+...' : null)
        return
      }

      setModifierHint(null)

      const keyMap: Record<string, string> = {
        ' ': 'Space',
        Tab: 'Tab',
        Enter: 'Enter',
        Backspace: 'Backspace',
        Escape: 'Escape',
        Delete: 'Delete',
        Insert: 'Insert',
        Home: 'Home',
        End: 'End',
        PageUp: 'PageUp',
        PageDown: 'PageDown',
        ArrowUp: 'Up',
        ArrowDown: 'Down',
        ArrowLeft: 'Left',
        ArrowRight: 'Right',
      }

      let keyName = keyMap[e.key] || e.key
      if (keyName.length === 1) keyName = keyName.toUpperCase()

      if (parts.length === 0 && !STANDALONE_KEYS.has(keyName)) return

      parts.push(keyName)
      const combo = parts.join('+')
      // Auto-save immediately — no confirmation step needed.
      if (autoConfirmTimer.current) clearTimeout(autoConfirmTimer.current)
      confirmHotkey(combo)
    },
    [confirmHotkey],
  )

  const handleKeyUp = useCallback(() => {
    setModifierHint(null)
  }, [])

  useEffect(() => {
    if (!recording) return
    window.addEventListener('keydown', handleKeyDown, true)
    window.addEventListener('keyup', handleKeyUp, true)

    let unlistenFn: (() => void) | undefined
    let unlistenCombo: (() => void) | undefined

    // Plain Fn/Globe pressed with no other modifier held.
    // Wait briefly for a sequential key (e.g. Fn then /) before saving as plain "Fn".
    listen('hotkey:fn_detected', () => {
      if (autoConfirmTimer.current) clearTimeout(autoConfirmTimer.current)
      setPending('Fn')
      setModifierHint('Fn+...')
      autoConfirmTimer.current = setTimeout(() => {
        setModifierHint(null)
        confirmHotkey('Fn')
      }, 600)
    }).then((fn) => {
      unlistenFn = fn
    })

    // Fn combo detected by CGEventTap — either Fn+modifier (simultaneous, instant)
    // or Fn+key (sequential, via keycode table). Save immediately.
    listen<string>('hotkey:fn_combo_detected', (event) => {
      if (autoConfirmTimer.current) clearTimeout(autoConfirmTimer.current)
      setModifierHint(null)
      confirmHotkey(event.payload)
    }).then((fn) => {
      unlistenCombo = fn
    })

    return () => {
      window.removeEventListener('keydown', handleKeyDown, true)
      window.removeEventListener('keyup', handleKeyUp, true)
      if (autoConfirmTimer.current) clearTimeout(autoConfirmTimer.current)
      unlistenFn?.()
      unlistenCombo?.()
      resumeHotkey().catch(() => {})
    }
  }, [recording, handleKeyDown, handleKeyUp, confirmHotkey])

  const handleClick = () => {
    if (recording && pending) {
      if (autoConfirmTimer.current) clearTimeout(autoConfirmTimer.current)
      confirmHotkey(pending)
    } else if (recording) {
      setRecording(false)
      setPending(null)
      setModifierHint(null)
      if (autoConfirmTimer.current) clearTimeout(autoConfirmTimer.current)
      resumeHotkey().catch(() => {})
    } else {
      pauseHotkey().catch(() => {})
      setRecording(true)
      setPending(null)
      setError(null)
    }
  }

  // While not recording: show chips if value exists, otherwise show placeholder button
  if (!recording) {
    return (
      <div>
        {value ? (
          <div
            className="flex items-center gap-2 cursor-pointer group"
            onClick={handleClick}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => e.key === 'Enter' && handleClick()}
          >
            <HotkeyChips
              hotkey={value}
              onClear={
                clearable
                  ? () => {
                      onSave('').catch(() => {})
                    }
                  : undefined
              }
            />
          </div>
        ) : (
          <button
            onClick={handleClick}
            className="px-3 py-1.5 rounded-[8px] text-[12px] text-text-tertiary border border-dashed border-border bg-bg-secondary hover:border-border-focus transition-colors cursor-pointer"
          >
            {t('settings.clickToSet', '点击设置')}
          </button>
        )}
        {error && <p className="text-[11px] text-error mt-1.5">{error}</p>}
      </div>
    )
  }

  return (
    <div>
      <button
        onClick={handleClick}
        className="px-3 py-2 rounded-[10px] text-[13px] font-mono text-left border border-text-secondary bg-bg-tertiary text-text-primary ring-2 ring-text-secondary/20 transition-colors cursor-pointer"
      >
        {pending || modifierHint || t('settings.pressKeyCombination')}
      </button>
      {pending && (
        <p className="text-[11px] text-text-tertiary mt-1.5">{t('settings.clickToConfirm')}</p>
      )}
      {!pending && (
        <p className="text-[11px] text-text-tertiary mt-1.5">
          {t('settings.pressKeyCombinationHint', 'Press any key or Fn / Globe')}
        </p>
      )}
      {error && <p className="text-[11px] text-error mt-1.5">{error}</p>}
    </div>
  )
}

export function GeneralPane() {
  const config = useAppStore((s) => s.config)
  const updateConfig = useAppStore((s) => s.updateConfig)
  const { t } = useTranslation()
  const isMac = navigator.platform.toLowerCase().includes('mac')
  const [accessibilityGranted, setAccessibilityGranted] = useState<boolean | null>(null)

  const handleCheckAccessibility = useCallback(() => {
    requestAccessibilityPermission().then((granted) => {
      setAccessibilityGranted(granted)
    })
  }, [])

  const handleOpenAccessibilitySettings = useCallback(() => {
    openAccessibilitySettings()
  }, [])

  const handleSaveHotkey = useCallback(
    async (hotkey: string) => {
      await updateHotkey(hotkey)
      updateConfig({ hotkey })
    },
    [updateConfig],
  )

  const handleSaveTranslateHotkey = useCallback(
    async (hotkey: string) => {
      await updateTranslateHotkey(hotkey)
      updateConfig({ translate_hotkey: hotkey })
    },
    [updateConfig],
  )

  return (
    <div className="space-y-6">
      {/* ── Keyboard Shortcuts ── */}
      <SectionHeader icon={<Keyboard size={14} />} title={t('settings.keyboardShortcuts', '键盘快捷键')} />

      <div className="bg-bg-secondary border border-border rounded-[12px] divide-y divide-border">
        {/* Voice input hotkey */}
        <div className="flex items-center justify-between px-4 py-3.5">
          <div>
            <p className="text-[13px] font-medium text-text-primary">
              {t('settings.voiceInputHotkey', '语音输入')}
            </p>
            <p className="text-[11px] text-text-tertiary mt-0.5">
              {t('settings.voiceInputHotkeyDesc', '按下开始和停止语音输入')}
            </p>
          </div>
          <HotkeyRecorder value={config.hotkey} onSave={handleSaveHotkey} />
        </div>

        {/* Translate hotkey */}
        <div className="flex items-center justify-between px-4 py-3.5">
          <div>
            <p className="text-[13px] font-medium text-text-primary">
              {t('settings.translateHotkey', '翻译')}
            </p>
            <p className="text-[11px] text-text-tertiary mt-0.5">
              {t('settings.translateHotkeyDesc', '按下开始和停止翻译。')}
            </p>
          </div>
          <HotkeyRecorder
            value={config.translate_hotkey ?? ''}
            onSave={handleSaveTranslateHotkey}
            clearable
          />
        </div>
      </div>

      {/* ── Language ── */}
      <SectionHeader icon={<Globe size={14} />} title={t('settings.languageSection', '语言')} />

      <div className="bg-bg-secondary border border-border rounded-[12px] divide-y divide-border">
        {/* UI Language */}
        <div className="flex items-center justify-between px-4 py-3.5">
          <div>
            <p className="text-[13px] font-medium text-text-primary">{t('settings.uiLanguage', '界面语言')}</p>
            <p className="text-[11px] text-text-tertiary mt-0.5">
              {t('settings.uiLanguageDesc', '选择用户界面使用的语言。')}
            </p>
          </div>
          <select
            value={config.ui_language ?? 'en'}
            onChange={(e) => {
              const lang = e.target.value as UiLanguage
              updateConfig({ ui_language: lang })
              i18n.changeLanguage(lang)
            }}
            className="px-3 py-2 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors min-w-[160px]"
          >
            <option value="en">English</option>
            <option value="zh">简体中文（中国大陆）</option>
          </select>
        </div>

        {/* Translate target language */}
        <div className="flex items-center justify-between px-4 py-3.5">
          <div>
            <p className="text-[13px] font-medium text-text-primary">
              {t('settings.translateTarget', '翻译目标')}
            </p>
            <p className="text-[11px] text-text-tertiary mt-0.5">
              {t('settings.translateTargetDesc', '选择翻译模式下的听写目标语言。')}
            </p>
          </div>
          <select
            value={config.target_lang}
            onChange={(e) => updateConfig({ target_lang: e.target.value })}
            className="px-3 py-2 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors min-w-[160px]"
          >
            {TARGET_LANGUAGES.map((l) => (
              <option key={l.value} value={l.value}>
                {l.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* ── Accessibility (macOS only) ── */}
      {isMac && (
        <Section title={t('settings.accessibility', 'Accessibility Permission')}>
          <div className="flex items-center justify-between">
            <p className="text-[13px] text-text-secondary">
              {accessibilityGranted === true
                ? t('settings.accessibilityGranted', 'Permission granted ✓')
                : t('settings.accessibilityDesc', 'Required for keyboard & auto-paste output')}
            </p>
            <div className="flex items-center gap-2">
              {accessibilityGranted !== true && (
                <button
                  onClick={handleOpenAccessibilitySettings}
                  className="jelly-btn px-3 py-1.5 rounded-lg text-[13px] text-text-primary border border-border"
                >
                  {t('settings.accessibilityGrant', 'Open Accessibility Settings')}
                </button>
              )}
              <button
                onClick={handleCheckAccessibility}
                className="jelly-btn-accent px-3 py-1.5 rounded-lg text-[13px] font-medium border-none"
              >
                {t('settings.accessibilityRecheck', 'Check Permission')}
              </button>
            </div>
          </div>
        </Section>
      )}

      {/* ── Max Recording Duration ── */}
      <Section title={t('settings.maxRecordingDuration', 'Max Recording Duration')}>
        <div className="flex items-center gap-3">
          <input
            type="range"
            min={10}
            max={300}
            step={10}
            value={config.max_recording_seconds}
            onChange={(e) => updateConfig({ max_recording_seconds: Number(e.target.value) })}
            className="flex-1 accent-accent"
          />
          <span className="text-[13px] text-text-secondary font-mono w-12 text-right">
            {config.max_recording_seconds}s
          </span>
        </div>
      </Section>

      {/* ── Other ── */}
      <Section title={t('settings.other')}>
        <div className="space-y-3">
          <Toggle
            checked={config.auto_start}
            onChange={(checked) => updateConfig({ auto_start: checked })}
            label={t('settings.launchAtStartup')}
          />
          {config.auto_start && (
            <Toggle
              checked={config.start_minimized}
              onChange={(checked) => updateConfig({ start_minimized: checked })}
              label={t('settings.startMinimized')}
            />
          )}
        </div>
      </Section>
    </div>
  )
}

function SectionHeader({ icon, title }: { icon: React.ReactNode; title: string }) {
  return (
    <div className="flex items-center gap-2 text-[12px] font-medium text-text-tertiary uppercase tracking-wider">
      {icon}
      <span>{title}</span>
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-[11px] font-medium text-text-tertiary uppercase tracking-wider mb-2">
        {title}
      </h3>
      <div className="bg-bg-secondary border border-border rounded-[12px] p-4">
        {children}
      </div>
    </div>
  )
}
