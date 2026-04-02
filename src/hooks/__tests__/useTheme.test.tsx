// @vitest-environment jsdom

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, cleanup } from '@testing-library/react'

const mockUseAppStore = vi.fn()

vi.mock('../../stores/appStore', () => ({
  useAppStore: (selector: (state: { config: { theme: 'light' | 'dark' | 'system' } }) => unknown) =>
    selector(mockUseAppStore()),
}))

import { useTheme } from '../useTheme'

function Harness() {
  useTheme()
  return null
}

describe('useTheme', () => {
  const originalMatchMedia = window.matchMedia

  beforeEach(() => {
    cleanup()
    document.documentElement.className = ''
    mockUseAppStore.mockReturnValue({ config: { theme: 'light' } })
  })

  afterEach(() => {
    window.matchMedia = originalMatchMedia
    cleanup()
    document.documentElement.className = ''
  })

  it('follows system dark mode even when config theme is light', () => {
    const addEventListener = vi.fn()
    const removeEventListener = vi.fn()

    window.matchMedia = vi.fn().mockReturnValue({
      matches: true,
      media: '(prefers-color-scheme: dark)',
      addEventListener,
      removeEventListener,
    }) as typeof window.matchMedia

    render(<Harness />)

    expect(document.documentElement.classList.contains('dark')).toBe(true)
  })
})
