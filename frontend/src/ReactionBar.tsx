import { useState } from 'react'
import { createPortal } from 'react-dom'
import { SmilePlus } from 'lucide-react'
import { forumApi, type EmojiAgg } from './api'

const ALLOWED = ['👍', '❤️', '😂', '😮', '😢', '🎉', '🚀', '👀']

/** Reaction chips + an add-reaction picker for a single post. */
export default function ReactionBar({ postId, initial }: { postId: string; initial: EmojiAgg[] }) {
  const [aggs, setAggs] = useState<EmojiAgg[]>(initial)
  const [pickerPos, setPickerPos] = useState<{ left: number; bottom: number } | null>(null)

  const toggle = async (emoji: string) => {
    setPickerPos(null)
    try {
      const r = await forumApi.react(postId, emoji)
      setAggs(r.reactions)
    } catch { /* ignore */ }
  }

  const openPicker = (e: React.MouseEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setPickerPos({ left: r.left, bottom: window.innerHeight - r.top + 6 })
  }

  return (
    <div className="flex items-center gap-1.5 mt-2 flex-wrap">
      {aggs.map(a => (
        <button key={a.emoji} onClick={() => toggle(a.emoji)}
          className={`flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border transition-colors
                      ${a.me ? 'bg-primary-light border-primary text-primary' : 'bg-surface-1 border-border text-text-secondary hover:bg-surface-2'}`}>
          <span className="text-sm leading-none">{a.emoji}</span>
          <span className="font-medium">{a.count}</span>
        </button>
      ))}
      <button onClick={openPicker} title="React"
        className="p-1 rounded-full text-text-tertiary hover:text-text-primary hover:bg-surface-2">
        <SmilePlus size={15} />
      </button>
      {pickerPos && createPortal(
        <>
          <div className="fixed inset-0 z-[9999]" onClick={() => setPickerPos(null)} />
          <div style={{ position: 'fixed', left: pickerPos.left, bottom: pickerPos.bottom, zIndex: 10000 }}
            className="bg-surface-0 border border-border rounded-lg shadow-xl p-1.5 flex gap-0.5">
            {ALLOWED.map(e => (
              <button key={e} onClick={() => toggle(e)} className="text-lg leading-none p-1 rounded hover:bg-surface-1">{e}</button>
            ))}
          </div>
        </>,
        document.body,
      )}
    </div>
  )
}
