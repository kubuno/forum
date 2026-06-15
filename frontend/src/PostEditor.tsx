import { useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { Bold, Italic, Link2, Image as ImageIcon, Smile } from 'lucide-react'
import { prompt } from '@kubuno/sdk'

const EMOJIS = ['😀', '😅', '😂', '🙂', '😉', '😍', '🤔', '👍', '👎', '🙏', '👏', '🔥', '🎉', '✅', '❌', '⚠️', '💡', '❤️', '🚀', '👀', '📌', '⏰', '📎', '🐛']

interface Props {
  value: string
  onChange: (v: string) => void
  onSubmit?: () => void
  placeholder?: string
  autoFocus?: boolean
  rows?: number
}

/** Markdown composer reused across new-topic and reply/edit flows. */
export default function PostEditor({ value, onChange, onSubmit, placeholder, autoFocus, rows = 4 }: Props) {
  const { t } = useTranslation('forum')
  const ref = useRef<HTMLTextAreaElement>(null)
  const emojiBtnRef = useRef<HTMLButtonElement>(null)
  const [emojiOpen, setEmojiOpen] = useState(false)
  const [emojiPos, setEmojiPos] = useState<{ left: number; bottom: number } | null>(null)

  const toggleEmoji = () => {
    if (emojiOpen) { setEmojiOpen(false); return }
    const r = emojiBtnRef.current?.getBoundingClientRect()
    if (r) setEmojiPos({ left: r.left, bottom: window.innerHeight - r.top + 6 })
    setEmojiOpen(true)
  }

  const surround = (before: string, after: string) => {
    const el = ref.current
    if (!el) return
    const s = el.selectionStart, e = el.selectionEnd
    const sel = value.slice(s, e)
    const next = value.slice(0, s) + before + sel + after + value.slice(e)
    onChange(next)
    requestAnimationFrame(() => { el.focus(); el.selectionStart = s + before.length; el.selectionEnd = e + before.length })
  }

  const insertAtCursor = (text: string) => {
    const el = ref.current
    const pos = el ? el.selectionStart : value.length
    onChange(value.slice(0, pos) + text + value.slice(pos))
    requestAnimationFrame(() => { if (el) { el.focus(); el.selectionStart = el.selectionEnd = pos + text.length } })
  }

  const insertLink = async () => {
    const url = await prompt({ title: t('insert_link'), placeholder: 'https://…', confirmLabel: t('add') })
    if (!url?.trim()) return
    const el = ref.current
    const sel = el ? value.slice(el.selectionStart, el.selectionEnd) : ''
    insertAtCursor(`[${sel || url.trim()}](${url.trim()})`)
  }

  const insertImage = async () => {
    const url = await prompt({ title: t('insert_image'), placeholder: 'https://…', confirmLabel: t('add') })
    if (url?.trim()) insertAtCursor(`![](${url.trim()})`)
  }

  const Btn = ({ onClick, title, children }: { onClick: () => void; title: string; children: React.ReactNode }) => (
    <button type="button" onMouseDown={(e) => e.preventDefault()} onClick={onClick} title={title}
      className="p-1.5 rounded text-text-secondary hover:text-text-primary hover:bg-surface-2">{children}</button>
  )

  return (
    <div className="border border-border rounded-lg overflow-visible relative bg-surface-0">
      <div className="flex items-center gap-0.5 px-1.5 py-1 border-b border-border">
        <Btn onClick={() => surround('**', '**')} title={t('bold')}><Bold size={15} /></Btn>
        <Btn onClick={() => surround('*', '*')} title={t('italic')}><Italic size={15} /></Btn>
        <Btn onClick={insertLink} title={t('insert_link')}><Link2 size={15} /></Btn>
        <Btn onClick={insertImage} title={t('insert_image')}><ImageIcon size={15} /></Btn>
        <button ref={emojiBtnRef} type="button" onMouseDown={(e) => e.preventDefault()} onClick={toggleEmoji} title={t('emoji')}
          className="p-1.5 rounded text-text-secondary hover:text-text-primary hover:bg-surface-2"><Smile size={15} /></button>
        {emojiOpen && emojiPos && createPortal(
          <>
            <div className="fixed inset-0 z-[9999]" onClick={() => setEmojiOpen(false)} />
            <div
              style={{ position: 'fixed', left: emojiPos.left, bottom: emojiPos.bottom, zIndex: 10000 }}
              className="bg-surface-0 border border-border rounded-lg shadow-xl p-1.5 grid grid-cols-8 gap-0.5 w-64">
              {EMOJIS.map(e => (
                <button key={e} type="button" onMouseDown={(ev) => ev.preventDefault()}
                  onClick={() => { insertAtCursor(e); setEmojiOpen(false) }}
                  className="text-lg leading-none p-1 rounded hover:bg-surface-1">{e}</button>
              ))}
            </div>
          </>,
          document.body,
        )}
      </div>
      <textarea
        ref={ref}
        value={value}
        autoFocus={autoFocus}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => { if (onSubmit && e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); onSubmit() } }}
        placeholder={placeholder}
        rows={rows}
        className="w-full text-sm p-2.5 resize-y focus:outline-none rounded-b-lg bg-surface-0"
      />
    </div>
  )
}
