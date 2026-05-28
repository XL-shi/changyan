import { Mic } from 'lucide-react'

export function CapsuleIdle() {
  return (
    <div className="w-9 h-9 flex items-center justify-center">
      <Mic size={14} className="text-neutral-400 cy-idle-pulse" strokeWidth={1.75} />
    </div>
  )
}
