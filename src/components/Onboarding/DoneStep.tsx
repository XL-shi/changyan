import { motion } from 'framer-motion'
import { Check, Keyboard, MousePointerClick, GripHorizontal, MousePointer } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'

export function DoneStep() {
  const { t } = useTranslation()
  const config = useAppStore((s) => s.config)

  return (
    <div className="flex flex-col items-center gap-5 py-2">
      {/* Success animation */}
      <motion.div
        className="w-16 h-16 rounded-full bg-success/10 flex items-center justify-center"
        initial={{ scale: 0 }}
        animate={{ scale: 1 }}
        transition={{ type: 'spring', stiffness: 500, damping: 20 }}
      >
        <motion.div
          initial={{ opacity: 0, scale: 0 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.2, type: 'spring', stiffness: 500, damping: 20 }}
        >
          <Check size={28} className="text-success" />
        </motion.div>
      </motion.div>

      <div className="text-center">
        <h2 className="text-[17px] font-semibold text-text-primary">{t('onboarding.allSet')}</h2>
        <p className="text-[13px] text-text-secondary mt-1">
          {t('onboarding.capsuleOnDesktop')}
        </p>
      </div>

      {/* Usage tips */}
      <div className="w-full space-y-2">
        <Tip
          icon={Keyboard}
          title={`${config.hotkey_mode === 'hold' ? t('onboarding.hold') : t('onboarding.press')} ${config.hotkey}`}
          desc={config.hotkey_mode === 'hold' ? t('onboarding.tipHotkeyHold') : t('onboarding.tipHotkeyToggle')}
        />
        <Tip
          icon={MousePointerClick}
          title={t('onboarding.tipClickCapsule')}
          desc={t('onboarding.tipClickCapsuleDesc')}
        />
        <Tip
          icon={GripHorizontal}
          title={t('onboarding.tipDrag')}
          desc={t('onboarding.tipDragDesc')}
        />
        <Tip
          icon={MousePointer}
          title={t('onboarding.tipRightClick')}
          desc={t('onboarding.tipRightClickDesc')}
        />
      </div>
    </div>
  )
}

function Tip({
  icon: Icon,
  title,
  desc,
}: {
  icon: React.ComponentType<{ size?: number; className?: string }>
  title: string
  desc: string
}) {
  return (
    <div className="flex items-center gap-3 px-3 py-2.5 bg-bg-secondary rounded-[10px]">
      <div className="p-1.5 rounded-[8px] bg-bg-tertiary text-text-tertiary shrink-0">
        <Icon size={14} />
      </div>
      <div>
        <p className="text-[13px] font-medium text-text-primary">{title}</p>
        <p className="text-[11px] text-text-tertiary">{desc}</p>
      </div>
    </div>
  )
}
