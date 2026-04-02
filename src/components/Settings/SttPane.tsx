import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useAppStore } from '../../stores/appStore'
import { useAuthStore } from '../../stores/authStore'
import { STT_PROVIDERS, LANGUAGES } from '../../lib/constants'
import {
  benchSttConnection,
  getSenseVoiceModelStatus,
  downloadSenseVoiceModel,
  deleteSenseVoiceModel,
  type ModelStatus,
  type ModelDownloadProgress,
} from '../../lib/tauri'
import { FormField } from './shared/FormField'
import { CheckCircle2, XCircle, Loader2, Crown, Download, Trash2, HardDrive } from 'lucide-react'

// macOS 13+ required for local SenseVoice inference (sherpa-onnx requires macOS 13.3+ libc++)
function isMacOsSupported(): boolean {
  const ua = navigator.userAgent
  // Tauri on macOS: "Mac OS X 10_15_7" or "Mac OS X 13_6" style
  const match = ua.match(/Mac OS X (\d+)[_.](\d+)/)
  if (!match) return true // non-macOS or unknown, let it try
  const major = parseInt(match[1], 10)
  const minor = parseInt(match[2], 10)
  // macOS 13 = "Mac OS X 13_x" in modern UA; older style "10_15" = Catalina (10.x)
  if (major === 10) return false // macOS 10.x (Catalina and below)
  if (major === 11) return false // macOS 11 Big Sur
  if (major === 12) return false // macOS 12 Monterey
  return major >= 13 || (major === 13 && minor >= 3)
}

function SenseVoiceLocalPanel() {
  const { t } = useTranslation()
  const [status, setStatus] = useState<ModelStatus | null>(null)
  const [downloading, setDownloading] = useState(false)
  const [progress, setProgress] = useState(0)
  const [downloadError, setDownloadError] = useState<string | null>(null)
  const macOsOk = isMacOsSupported()

  const refresh = async () => {
    const s = await getSenseVoiceModelStatus()
    setStatus(s)
  }

  useEffect(() => {
    refresh()
  }, [])

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined
    let unlistenComplete: (() => void) | undefined

    listen<ModelDownloadProgress>('model:download-progress', (e) => {
      setProgress(e.payload.percent)
    }).then((fn) => {
      unlistenProgress = fn
    })

    listen('model:download-complete', () => {
      setDownloading(false)
      setProgress(100)
      refresh()
    }).then((fn) => {
      unlistenComplete = fn
    })

    return () => {
      unlistenProgress?.()
      unlistenComplete?.()
    }
  }, [])

  const handleDownload = async () => {
    setDownloading(true)
    setProgress(0)
    setDownloadError(null)
    try {
      await downloadSenseVoiceModel()
    } catch (e) {
      setDownloading(false)
      setDownloadError(e instanceof Error ? e.message : String(e))
    }
  }

  const handleDelete = async () => {
    if (!window.confirm(t('settings.senseVoiceDeleteConfirm'))) return
    await deleteSenseVoiceModel()
    refresh()
  }

  return (
    <div className="border border-border rounded-[10px] p-4 space-y-3">
      <div className="flex items-center gap-2">
        <HardDrive size={14} className="text-text-secondary" />
        <span className="text-[13px] font-medium text-text-primary">
          {t('settings.localModel')}
        </span>
        <span className="text-[11px] text-text-tertiary">{t('settings.localModelHint')}</span>
      </div>

      {!macOsOk ? (
        <div className="space-y-1">
          <p className="text-[12px] text-text-secondary">{t('settings.localModelRequiresMacOs13')}</p>
          <p className="text-[11px] text-text-tertiary">{t('settings.localModelCurrentOs')}: macOS 12</p>
        </div>
      ) : status?.isDownloaded ? (
        <div className="space-y-2">
          <div className="flex items-center gap-1.5 text-[12px] text-success">
            <CheckCircle2 size={13} />
            <span>{t('settings.modelReady')}</span>
            {status.sizeMb !== null && (
              <span className="text-text-tertiary">({status.sizeMb.toFixed(0)} MB)</span>
            )}
          </div>
          <button
            onClick={handleDelete}
            className="flex items-center gap-1.5 text-[12px] text-error hover:underline cursor-pointer"
          >
            <Trash2 size={12} />
            {t('settings.deleteModel')}
          </button>
        </div>
      ) : downloading ? (
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-[12px] text-text-secondary">
            <Loader2 size={13} className="animate-spin" />
            <span>
              {t('settings.downloadingModel')} {progress.toFixed(0)}%
            </span>
          </div>
          <div className="w-full h-1.5 bg-bg-secondary rounded-full overflow-hidden">
            <div
              className="h-full bg-accent transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      ) : (
        <div className="space-y-2">
          <p className="text-[12px] text-text-secondary">{t('settings.modelNotDownloaded')}</p>
          {downloadError && <p className="text-[12px] text-error">{downloadError}</p>}
          <button
            onClick={handleDownload}
            className="jelly-btn-accent flex items-center gap-1.5 px-3 py-2 rounded-[8px] text-[12px] font-medium cursor-pointer border-none"
          >
            <Download size={13} />
            {t('settings.downloadModel')} (~360 MB)
          </button>
        </div>
      )}
    </div>
  )
}

export function SttPane() {
  const config = useAppStore((s) => s.config)
  const updateConfig = useAppStore((s) => s.updateConfig)
  const sttTestStatus = useAppStore((s) => s.sttTestStatus)
  const setSttTestStatus = useAppStore((s) => s.setSttTestStatus)
  const sttLatencyMs = useAppStore((s) => s.sttLatencyMs)
  const setSttLatencyMs = useAppStore((s) => s.setSttLatencyMs)
  const { user, plan } = useAuthStore()
  const { t } = useTranslation()

  const isCloud = config.stt_provider === 'cloud'
  const isLocalSenseVoice = config.stt_provider === 'sensevoice-local'

  const handleTest = async () => {
    setSttTestStatus('testing')
    setSttLatencyMs(null)
    try {
      const ms = await benchSttConnection(config.stt_api_key, config.stt_provider)
      console.log('[STT Test] Received latency:', ms, 'type:', typeof ms)
      setSttLatencyMs(ms)
      setSttTestStatus('success')
    } catch (err) {
      console.error('[STT Test] Error:', err)
      setSttTestStatus('error')
    }
  }

  return (
    <div className="space-y-6">
      <FormField label={t('settings.provider')}>
        <select
          value={config.stt_provider}
          onChange={(e) => {
            updateConfig({ stt_provider: e.target.value as typeof config.stt_provider })
            setSttTestStatus('idle')
            setSttLatencyMs(null)
          }}
          className="w-full px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
        >
          {STT_PROVIDERS.map((p) => (
            <option key={p.value} value={p.value}>
              {p.label}
            </option>
          ))}
        </select>
      </FormField>

      {isLocalSenseVoice ? (
        <SenseVoiceLocalPanel />
      ) : isCloud ? (
        <div className="border border-border rounded-[10px] px-3 py-3 space-y-2">
          <div className="flex items-center gap-2 text-[13px]">
            <Crown size={14} className="text-accent" />
            <span className="text-text-primary font-medium">{t('settings.cloudSttPro')}</span>
          </div>
          {!user ? (
            <p className="text-[12px] text-text-secondary">{t('settings.sttSignInHint')}</p>
          ) : plan !== 'pro' ? (
            <p className="text-[12px] text-text-secondary">{t('settings.sttUpgradeHint')}</p>
          ) : (
            <p className="text-[12px] text-green-500">{t('settings.sttProActive')}</p>
          )}
        </div>
      ) : (
        <FormField label={t('settings.apiKey')}>
          <div className="flex gap-2">
            <input
              type="password"
              value={config.stt_api_key}
              onChange={(e) => {
                updateConfig({ stt_api_key: e.target.value })
                setSttTestStatus('idle')
                setSttLatencyMs(null)
              }}
              placeholder={t('settings.enterApiKey')}
              className="flex-1 px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
            />
            <button
              onClick={handleTest}
              disabled={!config.stt_api_key || sttTestStatus === 'testing'}
              className="jelly-btn-accent px-4 py-2.5 rounded-[10px] text-[13px] font-medium border-none flex items-center gap-1.5"
            >
              {sttTestStatus === 'testing' && <Loader2 size={14} className="animate-spin" />}
              {t('settings.test')}
            </button>
          </div>
          {sttTestStatus === 'success' && (
            <p className="flex items-center gap-1 text-[12px] text-success mt-2">
              <CheckCircle2 size={13} />{' '}
              {sttLatencyMs !== null ? `${sttLatencyMs}ms` : t('settings.connectionSuccess')}
            </p>
          )}
          {sttTestStatus === 'error' && (
            <p className="flex items-center gap-1 text-[12px] text-error mt-2">
              <XCircle size={13} /> {t('settings.connectionFailed')}
            </p>
          )}
          <p className="text-[11px] text-text-tertiary mt-1.5">{t('settings.storedLocally')}</p>
        </FormField>
      )}

      <FormField label={t('settings.sttLanguage')}>
        <select
          value={config.stt_language}
          onChange={(e) => updateConfig({ stt_language: e.target.value })}
          className="w-full px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
        >
          {LANGUAGES.map((l) => (
            <option key={l.value} value={l.value}>
              {l.label}
            </option>
          ))}
        </select>
      </FormField>
    </div>
  )
}
