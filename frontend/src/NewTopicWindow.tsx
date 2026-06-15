import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { MessageSquarePlus } from 'lucide-react'
import { FloatingWindow, Button, Input } from '@ui'
import { forumApi, type ForumPerms, type TopicType } from './api'
import PostEditor from './PostEditor'
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
  const [busy, setBusy] = useState(false)
  const canPin = perms?.is_moderator || perms?.is_admin
  const canAttach = perms?.can_attach !== false

  const submit = async () => {
    if (!title.trim() || !body.trim() || busy) return
    setBusy(true)
    try {
      const { topic, post } = await forumApi.createTopic(forumId, {
        title: title.trim(), body_md: body.trim(), topic_type: type,
      })
      if (attachments.length) await saveAttachments(post.id, attachments)
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
