// @vitest-environment jsdom

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'
import React from 'react'

const mockState = {
  config: {
    theme: 'system' as const,
    auto_start: false,
  },
  savedConfig: {
    theme: 'light' as const,
    auto_start: false,
  },
  resetConfig: vi.fn(),
  setSavedConfig: vi.fn(),
}

vi.mock('framer-motion', () => ({
  motion: {
    div: ({ children, ...props }: React.HTMLAttributes<HTMLDivElement>) => <div {...props}>{children}</div>,
  },
}))

vi.mock('../../../../stores/appStore', () => ({
  useAppStore: (selector: (state: typeof mockState) => unknown) => selector(mockState),
}))

vi.mock('../../../../lib/tauri', () => ({
  updateConfig: vi.fn().mockResolvedValue(undefined),
  setAutoStart: vi.fn().mockResolvedValue(undefined),
}))

import { DirtyBar } from '../DirtyBar'

describe('DirtyBar', () => {
  beforeEach(() => {
    cleanup()
    vi.clearAllMocks()
  })

  it('uses shared accent button styling for Save action', () => {
    render(<DirtyBar />)

    expect(screen.getByText('Save').className).toContain('jelly-btn-accent')
  })
})
