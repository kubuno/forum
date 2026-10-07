import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { MessageSquarePlus, BarChart3 } from 'lucide-react'
import { FloatingWindow, Button, Input } from '@ui'
import { forumApi, type ForumPerms, type TopicType, type NewPoll } from './api'
import PostEditor from './PostEditor'
import PollComposer from './PollComposer'
import { AttachPicker, saveAttachments, type PendingAttachment } from './Attachments'

interface Props {
  forumId: string
  perms?: ForumPerms
  onClose: () => void
  onCreated: (topicId: string) => void
}

const TYPES: TopicType[] = ['normal', 'sticky', 'announcement', 'global']

export default function NewTopicWindow({ forumId, perms, onClose, onCreated }: Props) {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [type, setType] = useState<TopicType>('normal')
  const [attachments, setAttachments] = useState<PendingAttachment[]>([])
  const [poll, setPoll] = useState<NewPoll | null>(null)
  const [busy, setBusy] = useState(false)
  const canPin = perms?.is_moderator || perms?.is_admin
  const canAttach = perms?.can_attach !== false

  // Autosave: once the user has typed something non-trivial, persist a draft
  // in the background every 3s so the post survives an accidental close.
  // Failures are swallowed on purpose — a draft is a convenience, never a
  // blocker for typing.
  const draftIdRef = useRef<string | null>(null)
  const draftTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (draftTimerRef.current) clearTimeout(draftTimerRef.current)
    if (title.trim().length + body.trim().length <= 10) return
    draftTimerRef.current = setTimeout(() => {
      forumApi.saveDraft({ forum_id: forumId, title: title.trim(), body_md: body.trim() })
        .then(draft => { if (draft) draftIdRef.current = draft.id })
        .catch(() => {})
    }, 3000)
    return () => { if (draftTimerRef.current) clearTimeout(draftTimerRef.current) }
  }, [title, body, forumId])

  // A poll is attached only when it is complete: a question and at least two
  // non-empty options, mirroring what the server accepts.
  const cleanPoll = (): NewPoll | undefined => {
    if (!poll) return undefined
    const options = poll.options.map(o => o.trim()).filter(Boolean)
    if (!poll.question.trim() || options.length < 2) return undefined
    return { ...poll, question: poll.question.trim(), options }
  }

  const submit = async () => {
    if (!title.trim() || !body.trim() || busy) return
    setBusy(true)
    try {
      const { topic, post } = await forumApi.createTopic(forumId, {
        title: title.trim(), body_md: body.trim(), topic_type: type, poll: cleanPoll(),
      })
      if (attachments.length) await saveAttachments(post.id, attachments)
      if (draftIdRef.current) {
        forumApi.deleteDraft(draftIdRef.current).catch(() => {})
        draftIdRef.current = null
      }
      qc.invalidateQueries({ queryKey: ['forum-topics', forumId] })
      qc.invalidateQueries({ queryKey: ['forum-forums'] })
      onCreated(topic.id)
    } finally {
      setBusy(false)
    }
  }

  return (
    <FloatingWindow
      title={t('new_topic')}
      icon={<MessageSquarePlus size={18} />}
      onClose={onClose}
      defaultWidth={620}
      defaultHeight={520}
    >
      <div className="flex flex-col gap-3 p-4 h-full">
        <Input
          label={t('title')}
          value={title}
          autoFocus
          onChange={(e) => setTitle(e.target.value)}
          placeholder={t('title')}
        />
        {canPin && (
          <div>
            <label className="block text-xs font-medium text-text-secondary mb-1">{t('topic_type')}</label>
            <div className="flex gap-1.5 flex-wrap">
              {TYPES.map(ty => (
                <button key={ty} type="button" onClick={() => setType(ty)}
                  className={`px-2.5 py-1 rounded-md text-xs border ${type === ty
                    ? 'bg-primary text-white border-primary'
                    : 'border-border text-text-secondary hover:bg-surface-1'}`}>
                  {t(`type_${ty}`)}
                </button>
              ))}
            </div>
          </div>
        )}
        <div className="flex-1 min-h-0 flex flex-col">
          <label className="block text-xs font-medium text-text-secondary mb-1">{t('message')}</label>
          <PostEditor value={body} onChange={setBody} placeholder={t('write_first_post')} rows={8} />
        </div>
        {canAttach && <AttachPicker value={attachments} onChange={setAttachments} />}

        {/* Optional poll */}
        {poll === null ? (
          <button type="button" onClick={() => setPoll({ question: '', is_multiple: false, options: ['', ''], closes_at: null })}
            className="self-start inline-flex items-center gap-1.5 text-sm text-primary hover:underline">
            <BarChart3 size={15} /> {t('add_poll', { defaultValue: 'Add a poll' })}
          </button>
        ) : (
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-text-secondary">{t('poll', { defaultValue: 'Poll' })}</span>
              <button type="button" onClick={() => setPoll(null)} className="text-xs text-text-tertiary hover:text-danger">
                {t('remove_poll', { defaultValue: 'Remove poll' })}
              </button>
            </div>
            <PollComposer value={poll} onChange={setPoll} />
          </div>
        )}

        <div className="flex justify-end gap-2 pt-1">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} disabled={!title.trim() || !body.trim()} onClick={submit}>
            {t('create')}
          </Button>
        </div>
      </div>
    </FloatingWindow>
  )
}
