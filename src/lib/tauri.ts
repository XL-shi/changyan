import { invoke } from '@tauri-apps/api/core'
import type { AppConfig, HistoryEntry, DictionaryEntry } from '../stores/appStore'

// Pipeline commands
export async function startRecording(): Promise<void> {
  return invoke('start_recording')
}

export async function stopRecording(): Promise<void> {
  return invoke('stop_recording')
}

export async function forceIdle(): Promise<void> {
  return invoke('force_idle')
}

// Config commands
export async function getConfig(): Promise<AppConfig> {
  return invoke('get_config')
}

export async function updateConfig(config: AppConfig): Promise<void> {
  return invoke('update_config', { config })
}

// Connection test
export async function testSttConnection(apiKey: string, provider: string): Promise<boolean> {
  return invoke('test_stt_connection', { apiKey, provider })
}

export async function testLlmConnection(
  apiKey: string,
  provider: string,
  baseUrl: string,
  model: string,
): Promise<boolean> {
  return invoke('test_llm_connection', { apiKey, provider, baseUrl, model })
}

// Latency benchmark — returns round-trip time in milliseconds
export async function benchSttConnection(apiKey: string, provider: string): Promise<number> {
  return invoke('bench_stt_connection', { apiKey, provider })
}

export async function benchLlmConnection(
  apiKey: string,
  provider: string,
  baseUrl: string,
  model: string,
): Promise<number> {
  return invoke('bench_llm_connection', { apiKey, provider, baseUrl, model })
}

// LLM models
export async function fetchLlmModels(apiKey: string, baseUrl: string): Promise<string[]> {
  return invoke('fetch_llm_models', { apiKey, baseUrl })
}

// Hotkey
export async function updateHotkey(hotkey: string): Promise<void> {
  return invoke('update_hotkey', { hotkey })
}

export async function pauseHotkey(): Promise<void> {
  return invoke('pause_hotkey')
}

export async function resumeHotkey(): Promise<void> {
  return invoke('resume_hotkey')
}

export async function updateTranslateHotkey(hotkey: string): Promise<void> {
  return invoke('update_translate_hotkey', { hotkey })
}

export async function startRecordingTranslate(): Promise<void> {
  return invoke('start_recording_translate')
}

// History
export async function getHistory(limit: number, offset: number): Promise<HistoryEntry[]> {
  return invoke('get_history', { limit, offset })
}

export async function clearHistory(): Promise<void> {
  return invoke('clear_history')
}

// Dictionary
export async function getDictionary(): Promise<DictionaryEntry[]> {
  return invoke('get_dictionary')
}

export async function addDictionaryEntry(
  word: string,
  pronunciation: string | null,
): Promise<void> {
  return invoke('add_dictionary_entry', { word, pronunciation })
}

export async function removeDictionaryEntry(id: number): Promise<void> {
  return invoke('remove_dictionary_entry', { id })
}

// Auto-start
export async function setAutoStart(enabled: boolean): Promise<void> {
  return invoke('set_auto_start', { enabled })
}

// Onboarding persistence via tauri-plugin-store
export async function loadOnboardingCompleted(): Promise<boolean> {
  // Allow forcing onboarding in dev mode via VITE_INIT_ONBOARDING=true
  if (import.meta.env.VITE_INIT_ONBOARDING === 'true') return false
  try {
    const { load } = await import('@tauri-apps/plugin-store')
    const store = await load('settings.json')
    const val = await store.get<boolean>('onboarding_completed')
    return val === true
  } catch {
    return false
  }
}

export async function saveOnboardingCompleted(): Promise<void> {
  try {
    const { load } = await import('@tauri-apps/plugin-store')
    const store = await load('settings.json')
    await store.set('onboarding_completed', true)
  } catch (e) {
    console.error('Failed to persist onboarding state:', e)
  }
}


export async function requestAccessibilityPermission(): Promise<boolean> {
  return invoke('request_accessibility_permission')
}

export async function openAccessibilitySettings(): Promise<void> {
  return invoke('open_accessibility_settings')
}

// Model management (SenseVoice local)
export type ModelDownloadProgress = {
  downloaded: number
  total: number
  percent: number
}

export type ModelStatus = {
  isDownloaded: boolean
  modelDir: string
  sizeMb: number | null
}

export async function getSenseVoiceModelStatus(): Promise<ModelStatus> {
  return invoke('get_sensevoice_model_status')
}

export async function downloadSenseVoiceModel(): Promise<void> {
  return invoke('download_sensevoice_model')
}

export async function deleteSenseVoiceModel(): Promise<void> {
  return invoke('delete_sensevoice_model')
}

/** Returns the real OS version (e.g. "15.7.5"). Empty string on non-macOS. */
export async function getOsVersion(): Promise<string> {
  return invoke('get_os_version')
}
