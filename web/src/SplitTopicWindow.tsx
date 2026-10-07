import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Split } from 'lucide-react'
import { FloatingWindow, Button, Input, Dropdown } from '@ui'
import { forumApi } from './api'

interface Props {
  topicId: string
  currentForumId: string
  postIds: string[]
  onClose: () => void
  onSplit: (newTopicId: string) => void
}

export default function SplitTopicWindow({ topicId, currentForumId, postIds, onClose, onSplit }: Props) {
  const { t } = useTranslation('forum')
  const { data: forums = [] } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })
  const options = forums.map(f => ({ value: f.id, label: f.name }))
  const [title, setTitle] = useState('')
  const [forumId, setForumId] = useState(currentForumId)
  const [busy, setBusy] = useState(false)

  const submit = async () => {
    if (!title.trim() || postIds.length === 0 || busy) return
    setBusy(true)
    try {
      const newTopic = await forumApi.splitTopic(topicId, { post_ids: postIds, title: title.trim(), forum_id: forumId })
      onSplit(newTopic.id)
    } finally { setBusy(false) }
  }

  return (
    <FloatingWindow title={t('split_topic')} icon={<Split size={18} />} onClose={onClose} defaultWidth={440} defaultHeight={300}>
      <div className="p-4 flex flex-col gap-3 h-full">
        <p className="text-xs text-text-secondary">{t('post_count', { count: postIds.length })}</p>
        <Input label={t('title')} value={title} autoFocus onChange={(e) => setTitle(e.target.value)} placeholder={t('title')} />
        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1">{t('forums')}</label>
          <Dropdown options={options} value={forumId} onChange={setForumId} width="100%" height={36} />
        </div>
        <div className="flex justify-end gap-2 mt-auto">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} disabled={!title.trim() || postIds.length === 0} onClick={submit}>{t('split_topic')}</Button>
        </div>
      </div>
    </FloatingWindow>
  )
}
