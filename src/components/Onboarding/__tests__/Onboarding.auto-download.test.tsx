import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react'
import React from 'react'
import { useAppStore } from '../../../stores/appStore'

const tauriMocks = vi.hoisted(() => ({
  saveOnboardingCompleted: vi.fn().mockResolvedValue(undefined),
  updateConfig: vi.fn().mockResolvedValue(undefined),
  getSenseVoiceModelStatus: vi.fn().mockResolvedValue({
    isDownloaded: false,
    modelDir: '/tmp/model',
    sizeMb: null,
  }),
  downloadSenseVoiceModel: vi.fn().mockResolvedValue(undefined),
}))

afterEach(() => {
  cleanup()
})

const MOTION_PROPS = new Set([
  'initial',
  'animate',
  'exit',
  'transition',
  'variants',
  'whileHover',
  'whileTap',
  'whileFocus',
  'whileDrag',
  'whileInView',
  'layoutId',
  'layout',
  'drag',
  'dragConstraints',
  'onAnimationComplete',
])

vi.mock('framer-motion', () => ({
  AnimatePresence: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  motion: new Proxy(
    {},
    {
      get:
        (_t, tag: string) =>
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
          ({ children, ...rest }: any) => {
            const domProps: Record<string, unknown> = {}
            for (const [k, v] of Object.entries(rest)) {
              if (!MOTION_PROPS.has(k)) domProps[k] = v
            }
            return React.createElement(tag as string, { 'data-motion': tag, ...domProps }, children)
          },
    },
  ),
}))

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: () => {} },
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: 'zh' },
  }),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock('../../../lib/tauri', () => ({
  saveOnboardingCompleted: tauriMocks.saveOnboardingCompleted,
  updateConfig: tauriMocks.updateConfig,
  getSenseVoiceModelStatus: tauriMocks.getSenseVoiceModelStatus,
  downloadSenseVoiceModel: tauriMocks.downloadSenseVoiceModel,
  testLlmConnection: vi.fn().mockResolvedValue(true),
  fetchLlmModels: vi.fn().mockResolvedValue([]),
}))

import { Onboarding } from '../index'

function resetStore() {
  useAppStore.setState(useAppStore.getInitialState())
}

describe('Onboarding auto-download local model', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    useAppStore.setState({
      onboardingStep: 2,
      llmTestStatus: 'success',
      config: {
        ...useAppStore.getState().config,
        stt_provider: 'sensevoice-local',
      },
    })
  })

  it('completing onboarding downloads local model before finishing', async () => {
    render(<Onboarding />)

    fireEvent.click(screen.getByRole('button', { name: 'onboarding.getStarted' }))

    await waitFor(() => {
      expect(tauriMocks.getSenseVoiceModelStatus).toHaveBeenCalledTimes(1)
      expect(tauriMocks.downloadSenseVoiceModel).toHaveBeenCalledTimes(1)
      expect(tauriMocks.saveOnboardingCompleted).toHaveBeenCalledTimes(1)
    })

    expect(tauriMocks.downloadSenseVoiceModel.mock.invocationCallOrder[0]).toBeLessThan(
      tauriMocks.saveOnboardingCompleted.mock.invocationCallOrder[0],
    )
    expect(useAppStore.getState().onboardingCompleted).toBe(true)
  })

  it('skips download when local model is already present', async () => {
    tauriMocks.getSenseVoiceModelStatus.mockResolvedValueOnce({
      isDownloaded: true,
      modelDir: '/tmp/model',
      sizeMb: 360,
    })

    render(<Onboarding />)

    fireEvent.click(screen.getByRole('button', { name: 'onboarding.getStarted' }))

    await waitFor(() => {
      expect(tauriMocks.saveOnboardingCompleted).toHaveBeenCalledTimes(1)
    })

    expect(tauriMocks.downloadSenseVoiceModel).not.toHaveBeenCalled()
    expect(useAppStore.getState().onboardingCompleted).toBe(true)
  })
})
