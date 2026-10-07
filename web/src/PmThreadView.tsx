import { useEffect, useMemo, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronLeft, Send, Trash2, ShieldOff, Frown } from 'lucide-react'
import { Spinner, Button, ConfirmDialog } from '@ui'
import { useConfirm, useAuthStore } from '@kubuno/sdk'
import { forumApi, type PmMessage } from './api'
import { useResolveUsers } from './users'
import { AuthorName, AuthorAvatar } from './Author'
import { timeAgo, shortDateTime } from './helpers'
import PostBody from './PostBody'
import PostEditor from './PostEditor'

export default function PmThreadView() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const qc = useQueryClient()
  const { id: threadId = '' } = useParams()
  const me = useAuthStore(s => s.user)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  const [reply, setReply] = useState('')
  const [posting, setPosting] = useState(false)
  const bottomRef = useRef<HTMLDivElement>(null)

  // `retry: false` so an unauthorized/nonexistent thread id falls into the
  // "not found" branch below promptly instead of hammering the endpoint.
  const { data, isLoading, isError } = useQuery({
    queryKey: ['forum-pm-thread', threadId],
    queryFn: () => forumApi.getPmThread(threadId),
    enabled: !!threadId,
    retry: false,
  })

  const thread = data?.thread
  const messages = useMemo(() => data?.messages ?? [], [data])
  const others = useMemo(() => (thread?.participant_ids ?? []).filter(id => id !== me?.id), [thread, me])
  useResolveUsers(thread?.participant_ids ?? [])
  const isOneToOne = others.length === 1

  // Best-effort mark-as-read at mount — failures (e.g. the thread turning out
  // to be inaccessible) are swallowed, the "not found" branch handles that.
  useEffect(() => {
    if (!threadId) return
    forumApi.markPmRead(threadId).then(() => {
      qc.invalidateQueries({ queryKey: ['forum-pm-threads'] })
      qc.invalidateQueries({ queryKey: ['forum-pm-unread'] })
    }).catch(() => {})
  }, [threadId, qc])

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: 'end' })
  }, [messages.length])

  const submitReply = async () => {
    if (!reply.trim() || posting || !threadId) return
    setPosting(true)
    try {
      await forumApi.sendPmMessage(threadId, reply.trim())
      setReply('')
      qc.invalidateQueries({ queryKey: ['forum-pm-thread', threadId] })
      qc.invalidateQueries({ queryKey: ['forum-pm-threads'] })
    } finally {
      setPosting(false)
    }
  }

  const removeThread = async () => {
    if (!(await confirm({
      title: t('delete_conversation', { defaultValue: 'Supprimer la conversation' }),
      message: t('confirm_delete_conversation', { defaultValue: 'Supprimer définitivement cette conversation ?' }),
      confirmLabel: t('delete'),
      variant: 'danger',
    }))) return
    await forumApi.deletePmThread(threadId)
    qc.invalidateQueries({ queryKey: ['forum-pm-threads'] })
    navigate('/forum/pm')
  }

  const blockOther = async () => {
    const otherId = others[0]
    if (!otherId) return
    if (!(await confirm({
      title: t('block_user', { defaultValue: 'Bloquer cet utilisateur' }),
      message: t('confirm_block_user', { defaultValue: 'Il ne pourra plus vous envoyer de messages privés.' }),
      confirmLabel: t('block', { defaultValue: 'Bloquer' }),
      variant: 'danger',
    }))) return
    await forumApi.blockPmUser(otherId)
  }

  if (isLoading) return <div className="h-full flex items-center justify-center"><Spinner size="lg" /></div>

  if (isError || !thread) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-3 text-text-secondary">
        <Frown size={28} className="text-text-tertiary" />
        {t('pm_not_found', { defaultValue: 'Conversation introuvable.' })}
        <Button variant="ghost" size="sm" onClick={() => navigate('/forum/pm')}>{t('back')}</Button>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      <div className="max-w-3xl w-full mx-auto px-4 py-3 flex items-center gap-2 border-b border-border no-print">
        <button onClick={() => navigate('/forum/pm')} className="p-1.5 rounded hover:bg-surface-1 text-text-secondary" title={t('back')}>
          <ChevronLeft size={18} />
        </button>
        <div className="min-w-0 flex-1">
          <div className="font-semibold text-text-primary truncate">
            {thread.subject?.trim() || <OtherNames ids={others} />}
          </div>
          {thread.subject?.trim() && others.length > 0 && (
            <div className="text-xs text-text-tertiary truncate"><OtherNames ids={others} /></div>
          )}
        </div>
        {isOneToOne && (
          <button onClick={blockOther} className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary" title={t('block_user', { defaultValue: 'Bloquer' })}>
            <ShieldOff size={16} />
          </button>
        )}
        <button onClick={removeThread} className="p-2 rounded-lg hover:bg-danger-light text-text-secondary hover:text-danger" title={t('delete_conversation', { defaultValue: 'Supprimer la conversation' })}>
          <Trash2 size={16} />
        </button>
      </div>

      <div className="flex-1 overflow-auto">
        <div className="max-w-3xl mx-auto px-4 py-4 flex flex-col gap-3">
          {messages.map(m => <MessageBubble key={m.id} message={m} mine={m.sender_id === me?.id} />)}
          <div ref={bottomRef} />
        </div>
      </div>

      <div className="max-w-3xl w-full mx-auto px-4 py-3 border-t border-border no-print">
        <PostEditor value={reply} onChange={setReply} placeholder={t('write_message', { defaultValue: 'Écrivez votre message…' })} rows={3} onSubmit={submitReply} />
        <div className="flex justify-end mt-2">
          <Button variant="primary" icon={<Send size={15} />} loading={posting} disabled={!reply.trim()} onClick={submitReply}>
            {t('send')}
          </Button>
        </div>
      </div>

      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}

/** Comma-joined resolved names — a thin wrapper so each id can go through
 *  the `useUser` hook (via `AuthorName`) without breaking the rules of hooks
 *  in the caller, which only ever renders this as a whole component. */
function OtherNames({ ids }: { ids: string[] }) {
  return <>{ids.map((id, i) => <span key={id}>{i > 0 && ', '}<AuthorName id={id} /></span>)}</>
}

function MessageBubble({ message, mine }: { message: PmMessage; mine: boolean }) {
  // A light primary tint (not a solid fill) keeps PostBody's own fixed
  // text-text-primary color legible on the "mine" bubble too.
  return (
    <div className={`flex gap-2 ${mine ? 'flex-row-reverse' : ''}`}>
      <AuthorAvatar id={message.sender_id} size={32} />
      <div className={`max-w-[75%] rounded-2xl px-3 py-2 ${mine ? 'bg-primary-light' : 'bg-surface-1'}`}>
        <div className="text-xs text-text-tertiary mb-1">
          <AuthorName id={message.sender_id} /> · <span title={shortDateTime(message.created_at)}>{timeAgo(message.created_at)}</span>
        </div>
        <PostBody body={message.body_md} />
      </div>
    </div>
  )
}
