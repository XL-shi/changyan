import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor, act, cleanup } from '@testing-library/react'
import { GeneralPane } from '../GeneralPane'
import { useAppStore } from '../../../stores/appStore'

const tauriEventMocks = vi.hoisted(() => {
  const listeners = new Map<string, (event: { payload: unknown }) => void>()
  return { listeners }
})

vi.mock('react-i18next', () => ({
  initReactI18next: {
    type: '3rdParty',
    init: () => {},
  },
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: 'en' },
  }),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((eventName: string, callback: (event: { payload: unknown }) => void) => {
    tauriEventMocks.listeners.set(eventName, callback)
    return Promise.resolve(() => {
      tauriEventMocks.listeners.delete(eventName)
    })
  }),
}))

vi.mock('../../../lib/tauri', () => ({
  updateHotkey: vi.fn().mockResolvedValue(undefined),
  pauseHotkey: vi.fn().mockResolvedValue(undefined),
  resumeHotkey: vi.fn().mockResolvedValue(undefined),
  updateTranslateHotkey: vi.fn().mockResolvedValue(undefined),
  requestAccessibilityPermission: vi.fn().mockResolvedValue(true),
  openAccessibilitySettings: vi.fn().mockResolvedValue(undefined),
}))

afterEach(() => {
  cleanup()
})

function resetStore() {
  useAppStore.setState(useAppStore.getInitialState())
  useAppStore.getState().updateConfig({
    hotkey: '',
    translate_hotkey: '',
  })
}

function emitTauriEvent(eventName: string, payload?: unknown) {
  const listener = tauriEventMocks.listeners.get(eventName)
  if (!listener) throw new Error(`Missing listener for ${eventName}`)
  listener({ payload })
}

describe('GeneralPane Fn hotkey recorder', () => {
  beforeEach(() => {
    resetStore()
    tauriEventMocks.listeners.clear()
  })

  it('先按 Fn 再按左 Shift 时，语音输入快捷键保存为 Fn+LeftShift', async () => {
    const { updateHotkey } = await import('../../../lib/tauri')

    render(<GeneralPane />)
    fireEvent.click(screen.getAllByText('settings.clickToSet')[0])

    await waitFor(() => {
      expect(tauriEventMocks.listeners.has('hotkey:fn_detected')).toBe(true)
      expect(tauriEventMocks.listeners.has('hotkey:fn_combo_detected')).toBe(true)
    })

    await act(async () => {
      emitTauriEvent('hotkey:fn_detected')
      emitTauriEvent('hotkey:fn_combo_detected', 'Fn+LeftShift')
    })

    await waitFor(() => {
      expect(vi.mocked(updateHotkey)).toHaveBeenCalledWith('Fn+LeftShift')
      expect(useAppStore.getState().config.hotkey).toBe('Fn+LeftShift')
    })
  })

  it('先按 Fn 再按右 Shift 时，翻译快捷键保存为 Fn+RightShift', async () => {
    const { updateTranslateHotkey } = await import('../../../lib/tauri')

    render(<GeneralPane />)
    fireEvent.click(screen.getAllByText('settings.clickToSet')[1])

    await waitFor(() => {
      expect(tauriEventMocks.listeners.has('hotkey:fn_detected')).toBe(true)
      expect(tauriEventMocks.listeners.has('hotkey:fn_combo_detected')).toBe(true)
    })

    await act(async () => {
      emitTauriEvent('hotkey:fn_detected')
      emitTauriEvent('hotkey:fn_combo_detected', 'Fn+RightShift')
    })

    await waitFor(() => {
      expect(vi.mocked(updateTranslateHotkey)).toHaveBeenCalledWith('Fn+RightShift')
      expect(useAppStore.getState().config.translate_hotkey).toBe('Fn+RightShift')
    })
  })
})
