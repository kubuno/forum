import { useEffect, useRef, useState, type RefObject } from 'react'
import { createPortal } from 'react-dom'
import { Spinner } from '@ui'
import { forumApi, type UserBrief } from './api'

/** A mention inserted into a composer's text: the chosen user plus the exact
 *  "@label" substring that was written into the textarea, so the caller can
 *  later check the mention is still present before submitting (best-effort
 *  cleanup for mentions the author typed over or deleted by hand). */
export interface MentionRef {
  id: string
  label: string
}

/** Keeps only the mentions whose "@label" token is still found in `text`,
 *  deduplicated by user id. Used right before a post is submitted so a
 *  deleted/edited mention never reaches the server as `mention_user_ids`. */
export function resolveMentionIds(text: string, mentions: MentionRef[]): string[] {
  const ids = new Set<string>()
  for (const m of mentions) {
    if (text.includes(`@${m.label}`)) ids.add(m.id)
  }
  return [...ids]
}

interface Token {
  /** Index of the leading '@' in the text. */
  start: number
  query: string
}

/** Finds the `@query` token the caret currently sits inside, if any. The '@'
 *  must be at the start of the text or preceded by whitespace so plain
 *  emails (foo@bar) never trigger it. */
function detectToken(text: string, caret: number): Token | null {
  const upTo = text.slice(0, caret)
  const m = /(?:^|\s)@([^\s@]{0,32})$/.exec(upTo)
  if (!m) return null
  const query = m[1]
  return { start: caret - query.length - 1, query }
}

const MIRROR_PROPS = [
  'boxSizing', 'width', 'height', 'overflowX', 'overflowY',
  'borderTopWidth', 'borderRightWidth', 'borderBottomWidth', 'borderLeftWidth',
  'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
  'fontStyle', 'fontVariant', 'fontWeight', 'fontStretch', 'fontSize', 'lineHeight', 'fontFamily',
  'textAlign', 'textTransform', 'textIndent', 'textDecoration', 'letterSpacing', 'wordSpacing',
  'tabSize', 'whiteSpace', 'wordWrap', 'wordBreak',
] as const

/** Pixel offset of a character position inside a textarea, relative to the
 *  textarea's own padding box. Standard "mirror div" trick: an offscreen
 *  clone of the textarea holds the same text up to `pos`, and the position
 *  of a marker span appended right after it gives the caret's coordinates. */
function caretOffset(el: HTMLTextAreaElement, pos: number): { top: number; left: number; height: number } {
  const div = document.createElement('div')
  const style = window.getComputedStyle(el)
  div.style.position = 'absolute'
  div.style.visibility = 'hidden'
  div.style.whiteSpace = 'pre-wrap'
  div.style.wordWrap = 'break-word'
  const src = style as unknown as Record<string, string>
  const dst = div.style as unknown as Record<string, string>
  for (const p of MIRROR_PROPS) dst[p] = src[p]
  div.style.width = style.width
  div.textContent = el.value.slice(0, pos)
  const marker = document.createElement('span')
  marker.textContent = el.value.slice(pos) || '.'
  div.appendChild(marker)
  document.body.appendChild(div)
  const top = marker.offsetTop
  const left = marker.offsetLeft
  document.body.removeChild(div)
  return { top, left, height: parseInt(style.lineHeight, 10) || 18 }
}

interface Opts {
  value: string
  onChange: (v: string) => void
  textareaRef: RefObject<HTMLTextAreaElement | null>
  /** Called once per user picked from the dropdown; the caller collects
   *  these to build `mention_user_ids` when the post is submitted. */
  onMention?: (mention: MentionRef) => void
}

/** Detects an "@query" token being typed in a textarea, looks up matching
 *  users (debounced), and lets the caller insert "@display_name " on
 *  selection. Renders its own dropdown via a portal, positioned at the
 *  caret — nothing is rendered when no token is active. */
export function useMentionPicker({ value, onChange, textareaRef, onMention }: Opts) {
  const [token, setToken] = useState<Token | null>(null)
  const [results, setResults] = useState<UserBrief[]>([])
  const [loading, setLoading] = useState(false)
  const [activeIndex, setActiveIndex] = useState(0)
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const requestIdRef = useRef(0)

  const close = () => {
    setToken(null)
    setResults([])
    setPos(null)
    if (debounceRef.current) clearTimeout(debounceRef.current)
  }

  // Track caret/value changes on the textarea to find the active "@token".
  useEffect(() => {
    const el = textareaRef.current
    if (!el) return
    const check = () => {
      const t = detectToken(el.value, el.selectionStart ?? el.value.length)
      if (!t) { close(); return }
      setToken(t)
      const c = caretOffset(el, t.start)
      const r = el.getBoundingClientRect()
      setPos({ left: r.left + c.left - el.scrollLeft, top: r.top + c.top - el.scrollTop + c.height + 4 })
    }
    el.addEventListener('input', check)
    el.addEventListener('keyup', check)
    el.addEventListener('click', check)
    return () => {
      el.removeEventListener('input', check)
      el.removeEventListener('keyup', check)
      el.removeEventListener('click', check)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Debounced search whenever the token's query changes.
  useEffect(() => {
    if (!token || token.query.trim().length === 0) { setResults([]); setLoading(false); return }
    if (debounceRef.current) clearTimeout(debounceRef.current)
    const q = token.query.trim()
    const myRequest = ++requestIdRef.current
    setLoading(true)
    debounceRef.current = setTimeout(() => {
      forumApi.searchUsers(q)
        .then(users => { if (requestIdRef.current === myRequest) setResults(users) })
        .catch(() => { if (requestIdRef.current === myRequest) setResults([]) })
        .finally(() => { if (requestIdRef.current === myRequest) setLoading(false) })
    }, 200)
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current) }
  }, [token?.query])

  useEffect(() => { setActiveIndex(0) }, [results])

  const select = (user: UserBrief) => {
    const el = textareaRef.current
    if (!token) return
    const label = user.display_name || user.username
    const end = token.start + 1 + token.query.length
    const next = value.slice(0, token.start) + `@${label} ` + value.slice(end)
    onChange(next)
    onMention?.({ id: user.id, label })
    close()
    const caret = token.start + label.length + 2
    requestAnimationFrame(() => { if (el) { el.focus(); el.selectionStart = el.selectionEnd = caret } })
  }

  const open = !!token && !!pos && (loading || results.length > 0)

  /** Intercepts navigation/selection keys while the dropdown is open.
   *  Returns true when it handled the key, so the caller should stop
   *  further processing (e.g. skip its own submit-on-Enter shortcut). */
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>): boolean => {
    if (!open || results.length === 0) return false
    if (e.key === 'ArrowDown') { e.preventDefault(); setActiveIndex(i => (i + 1) % results.length); return true }
    if (e.key === 'ArrowUp') { e.preventDefault(); setActiveIndex(i => (i - 1 + results.length) % results.length); return true }
    if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); select(results[activeIndex]); return true }
    if (e.key === 'Escape') { e.preventDefault(); close(); return true }
    return false
  }

  const picker = open && pos ? createPortal(
    <>
      <div className="fixed inset-0 z-[9999]" onMouseDown={close} />
      <div
        style={{ position: 'fixed', left: pos.left, top: pos.top, zIndex: 10000 }}
        className="w-64 max-h-60 overflow-auto bg-surface-0 border border-border rounded-lg shadow-xl py-1">
        {loading && results.length === 0 ? (
          <div className="px-3 py-3 flex justify-center"><Spinner size="sm" /></div>
        ) : results.length === 0 ? null : results.map((u, i) => (
          <button key={u.id} type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => select(u)}
            className={`w-full flex items-center gap-2 px-2.5 py-1.5 text-left ${i === activeIndex ? 'bg-surface-2' : 'hover:bg-surface-1'}`}>
            {u.avatar_url ? (
              <img src={u.avatar_url} alt="" width={22} height={22} className="rounded-full object-cover shrink-0" />
            ) : (
              <div className="w-[22px] h-[22px] rounded-full bg-primary-light text-primary flex items-center justify-center shrink-0 text-[10px] font-semibold">
                {(u.display_name || u.username).slice(0, 2).toUpperCase()}
              </div>
            )}
            <span className="min-w-0 flex-1">
              <span className="block text-sm text-text-primary truncate">{u.display_name || u.username}</span>
              {u.display_name && <span className="block text-xs text-text-tertiary truncate">@{u.username}</span>}
            </span>
          </button>
        ))}
      </div>
    </>,
    document.body,
  ) : null

  return { picker, handleKeyDown }
}
