import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { Info, X } from 'lucide-react'
import { useState } from 'react'

export function CopyDialog({ noOverlay = false }: { noOverlay?: boolean }) {
  const { t } = useTranslation()
  const copyReadyText = useAppStore((s) => s.copyReadyText)
  const setCopyReadyText = useAppStore((s) => s.setCopyReadyText)
  const [copied, setCopied] = useState(false)

  if (!copyReadyText) return null

  const handleCopy = async () => {
    try {
      await writeText(copyReadyText)
      setCopied(true)
      setTimeout(() => {
        setCopied(false)
        setCopyReadyText(null)
      }, 1200)
    } catch {
      // clipboard already has the text from Rust side
      setCopied(true)
      setTimeout(() => {
        setCopied(false)
        setCopyReadyText(null)
      }, 1200)
    }
  }

  return (
    <div
      className={
        noOverlay
          ? 'flex items-center justify-center w-full h-full'
          : 'fixed inset-0 z-50 flex items-center justify-center bg-black/40'
      }
    >
      <div className="bg-bg-elevated text-text-primary rounded-2xl shadow-2xl w-[340px] p-5 relative border border-border">
        <button
          onClick={() => setCopyReadyText(null)}
          className="absolute top-3.5 right-3.5 text-text-tertiary hover:text-text-primary transition-colors border-none bg-transparent cursor-pointer p-1"
        >
          <X size={16} />
        </button>

        <div className="flex items-center gap-2 mb-3">
          <Info size={18} className="text-blue-400 shrink-0" />
          <span className="text-[15px] font-semibold">
            {t('copyDialog.title', '复制最后的转录')}
          </span>
        </div>

        <p className="text-[13px] text-text-secondary mb-4 line-clamp-3 bg-bg-secondary rounded-xl px-3 py-2 border border-border">
          &ldquo;{copyReadyText}&rdquo;
        </p>

        <button
          onClick={handleCopy}
          className="jelly-btn-accent w-full py-2.5 rounded-xl text-[14px] font-medium border-none"
        >
          {copied ? t('copyDialog.copied', '已复制') : t('copyDialog.copy', '复制')}
        </button>
      </div>
    </div>
  )
}
