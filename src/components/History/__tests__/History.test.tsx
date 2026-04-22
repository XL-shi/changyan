// @vitest-environment jsdom

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react'
import { History } from '../index'

const { mockClearHistory, mockDialogConfirm, mockSetHistory } = vi.hoisted(() => ({
  mockClearHistory: vi.fn(),
  mockDialogConfirm: vi.fn(),
  mockSetHistory: vi.fn(),
}))

const mockAppStore = {
  history: [
    {
      id: 1,
      created_at: '2026-04-02T22:10:00',
      app_name: 'WeChat',
      app_type: 'chat',
      raw_text: '你好',
      polished_text: '你好，能听见吗？',
      language: 'zh',
      duration_ms: 1000,
    },
  ],
  setHistory: mockSetHistory,
}

vi.mock('../../../lib/tauri', () => ({
  clearHistory: mockClearHistory,
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  confirm: mockDialogConfirm,
}))

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'history.title': 'History',
        'history.searchPlaceholder': 'Search history...',
        'history.clearAll': 'Clear all history',
        'history.clearConfirm': 'Are you sure you want to clear all history?',
        'history.failedToClear': 'Failed to clear history',
        'history.failedToCopy': 'Failed to copy',
        'history.today': 'Today',
        'history.yesterday': 'Yesterday',
        'history.copied': 'Copied',
      }
      return translations[key] || key
    },
  }),
}))

vi.mock('framer-motion', () => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  AnimatePresence: ({ children }: { children?: any }) => children,
  motion: {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    div: ({ children, whileHover: _whileHover, whileTap: _whileTap, transition: _transition, ...props }: Record<string, any>) =>
      <div {...props}>{children}</div>,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    button: ({ children, whileHover: _whileHover, whileTap: _whileTap, transition: _transition, ...props }: Record<string, any>) =>
      <button {...props}>{children}</button>,
  },
}))

vi.mock('../../Toast', () => ({
  toast: {
    error: vi.fn(),
  },
}))

vi.mock('../../../stores/appStore', () => ({
  useAppStore: (selector: (state: typeof mockAppStore) => unknown) => selector(mockAppStore),
}))

describe('History clear action', () => {
  const originalConfirm = window.confirm

  beforeEach(() => {
    vi.clearAllMocks()
    mockAppStore.history = [
      {
        id: 1,
        created_at: '2026-04-02T22:10:00',
        app_name: 'WeChat',
        app_type: 'chat',
        raw_text: '你好',
        polished_text: '你好，能听见吗？',
        language: 'zh',
        duration_ms: 1000,
      },
    ]
    mockSetHistory.mockReset()
    mockDialogConfirm.mockResolvedValue(true)
    mockClearHistory.mockResolvedValue(undefined)
    window.confirm = vi.fn(() => {
      throw new Error('window.confirm is unavailable in Tauri webview')
    })
  })

  afterEach(() => {
    cleanup()
    window.confirm = originalConfirm
  })

  it('uses the Tauri dialog confirm flow when browser confirm is unavailable', async () => {
    render(<History />)

    fireEvent.click(screen.getByRole('button', { name: /clear all history/i }))

    await waitFor(() => {
      expect(mockDialogConfirm).toHaveBeenCalledWith('Are you sure you want to clear all history?', {
        kind: 'warning',
      })
    })

    await waitFor(() => {
      expect(mockClearHistory).toHaveBeenCalledTimes(1)
      expect(mockSetHistory).toHaveBeenCalledWith([])
    })
  })
})
