import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Combine } from 'lucide-react'
import { FloatingWindow, Button, Spinner } from '@ui'
import { forumApi } from './api'
import { timeAgo } from './helpers'

interface Props {
  topicId: string
  forumId: string
  onClose: () => void
  onMerged: () => void
}

/** Merge another topic of the same forum into the current one. */
export default function MergeTopicWindow({ topicId, forumId, onClose, onMerged }: Props) {
  const { t } = useTranslation('forum')
  const { data, isLoading } = useQuery({ queryKey: ['forum-topics', forumId], queryFn: () => forumApi.listTopics(forumId) })
  const [selected, setSelected] = useState('')
  const [busy, setBusy] = useState(false)
  const candidates = (data?.topics ?? []).filter(tp => tp.id !== topicId)

  const submit = async () => {
    if (!selected || busy) return
    setBusy(true)
    try { await forumApi.mergeTopic(topicId, selected); onMerged() } finally { setBusy(false) }
  }

  return (
    <FloatingWindow title={t('merge_topic')} icon={<Combine size={18} />} onClose={onClose} defaultWidth={460} defaultHeight={420}>
      <div className="p-4 flex flex-col gap-3 h-full">
        <div className="flex-1 min-h-0 overflow-auto rounded-lg border border-border">
          {isLoading ? (
            <div className="p-6 flex justify-center"><Spinner /></div>
          ) : candidates.length === 0 ? (
            <div className="p-6 text-center text-sm text-text-secondary">{t('no_topics')}</div>
          ) : (
            <ul className="divide-y divide-border">
              {candidates.map(tp => (
                <li key={tp.id}>
                  <button onClick={() => setSelected(tp.id)}
                    className={`w-full text-left px-3 py-2 hover:bg-surface-1 ${selected === tp.id ? 'bg-primary-light' : ''}`}>
                    <div className="text-sm font-medium text-text-primary truncate">{tp.title}</div>
                    <div className="text-xs text-text-tertiary">{timeAgo(tp.last_post_at ?? tp.created_at)} · {t('post_count', { count: tp.reply_count + 1 })}</div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} disabled={!selected} onClick={submit}>{t('merge_topic')}</Button>
        </div>
      </div>
    </FloatingWindow>
  )
}
