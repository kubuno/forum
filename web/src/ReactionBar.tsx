import { useState } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { SmilePlus } from 'lucide-react'
import { Tooltip } from '@ui'
import { forumApi, type EmojiAgg } from './api'
import { useResolveUsers, userLabel, useUser } from './users'

const ALLOWED = ['👍', '❤️', '😂', '😮', '😢', '🎉', '🚀', '👀']

/** Reaction chips + an add-reaction picker for a single post. */
export default function ReactionBar({ postId, initial }: { postId: string; initial: EmojiAgg[] }) {
  const { t } = useTranslation('forum')
  const [aggs, setAggs] = useState<EmojiAgg[]>(initial)
  const [pickerPos, setPickerPos] = useState<{ left: number; bottom: number } | null>(null)
  // "Who reacted" is resolved lazily on first hover/click of any chip on this
  // post, not for every post of the topic up front — `null` until then.
  const [reactionUsers, setReactionUsers] = useState<Record<string, string[]> | null>(null)
  const [loadingUsers, setLoadingUsers] = useState(false)

  const ensureReactionUsers = () => {
    if (reactionUsers || loadingUsers) return
    setLoadingUsers(true)
    forumApi.postReactionUsers(postId)
      .then(list => setReactionUsers(Object.fromEntries(list.map(r => [r.emoji, r.users]))))
      .catch(() => setReactionUsers({}))
      .finally(() => setLoadingUsers(false))
  }

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
        <Tooltip key={a.emoji}
          label={loadingUsers && !reactionUsers ? t('loading', { defaultValue: 'Chargement…' })
            : reactionUsers ? <ReactionUsersLabel ids={reactionUsers[a.emoji] ?? []} /> : null}>
          <button onClick={() => toggle(a.emoji)} onMouseEnter={ensureReactionUsers}
            className={`flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border transition-colors
                        ${a.me ? 'bg-primary-light border-primary text-primary' : 'bg-surface-1 border-border text-text-secondary hover:bg-surface-2'}`}>
            <span className="text-sm leading-none">{a.emoji}</span>
            <span className="font-medium">{a.count}</span>
          </button>
        </Tooltip>
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

/** Resolved, comma-separated display names for a "who reacted" tooltip. */
function ReactionUsersLabel({ ids }: { ids: string[] }) {
  const { t } = useTranslation('forum')
  useResolveUsers(ids)
  if (!ids.length) return t('no_reactions', { defaultValue: 'Personne' })
  return (
    <>
      {ids.map((id, i) => <UserNameChunk key={id} id={id} separator={i > 0} />)}
    </>
  )
}

function UserNameChunk({ id, separator }: { id: string; separator: boolean }) {
  const u = useUser(id)
  return <>{separator ? ', ' : ''}{userLabel(u)}</>
}
