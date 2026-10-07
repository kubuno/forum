import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { MoveRight } from 'lucide-react'
import { FloatingWindow, Button, Dropdown } from '@ui'
import { forumApi } from './api'

interface Props {
  topicId: string
  currentForumId: string
  onClose: () => void
  onMoved: () => void
}

export default function MoveTopicWindow({ topicId, currentForumId, onClose, onMoved }: Props) {
  const { t } = useTranslation('forum')
  const { data: forums = [] } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })
  const options = forums.filter(f => f.id !== currentForumId).map(f => ({ value: f.id, label: f.name }))
  const [target, setTarget] = useState('')
  const [busy, setBusy] = useState(false)

  const submit = async () => {
    if (!target || busy) return
    setBusy(true)
    try { await forumApi.moveTopic(topicId, target); onMoved() } finally { setBusy(false) }
  }

  return (
    <FloatingWindow title={t('move_topic')} icon={<MoveRight size={18} />} onClose={onClose} defaultWidth={420} defaultHeight={240}>
      <div className="p-4 flex flex-col gap-3 h-full">
        <label className="text-xs font-medium text-text-secondary">{t('choose_target_forum')}</label>
        <Dropdown options={options} value={target} onChange={setTarget} placeholder={t('forums')} width="100%" height={36} />
        <div className="flex justify-end gap-2 mt-auto">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} disabled={!target} onClick={submit}>{t('move_topic')}</Button>
        </div>
      </div>
    </FloatingWindow>
  )
}
