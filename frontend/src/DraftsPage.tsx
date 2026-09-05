import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { FileText, Trash2 } from 'lucide-react'
import { Spinner, Button, ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi, type Draft } from './api'
import { timeAgo } from './helpers'

/** Strips the lightest Markdown markup so the list shows a plain-text excerpt. */
function excerptOf(bodyMd: string): string {
  return bodyMd.replace(/[#*_`>~-]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, 160)
}

export default function DraftsPage() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  const { data: drafts = [], isLoading } = useQuery({
    queryKey: ['forum-drafts'],
    queryFn: forumApi.listDrafts,
  })
  const { data: forums = [] } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })

  const resume = (draft: Draft) => {
    if (draft.topic_id) navigate(`/forum/topics/${draft.topic_id}`)
    else if (draft.forum_id) navigate(`/forum/forums/${draft.forum_id}?new=1`)
  }

  const remove = async (draft: Draft) => {
    if (!(await confirm({
      title: t('delete_draft', { defaultValue: 'Supprimer le brouillon' }),
      message: t('confirm_delete_draft', { defaultValue: 'Supprimer ce brouillon ?' }),
      confirmLabel: t('delete'),
      variant: 'danger',
    }))) return
    await forumApi.deleteDraft(draft.id)
    qc.setQueryData<Draft[]>(['forum-drafts'], prev => (prev ?? []).filter(d => d.id !== draft.id))
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-lg font-semibold text-text-primary">{t('drafts', { defaultValue: 'Brouillons' })}</h1>
          <span className="text-xs text-text-tertiary">{t('draft_count', { defaultValue: '{{count}} brouillons', count: drafts.length })}</span>
        </div>

        <div className="rounded-xl border border-border overflow-hidden bg-surface-0">
          {isLoading ? (
            <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
          ) : drafts.length === 0 ? (
            <div className="px-4 py-16 text-center text-text-secondary">{t('no_drafts', { defaultValue: 'Aucun brouillon pour l’instant.' })}</div>
          ) : (
            <ul className="divide-y divide-border">
              {drafts.map(d => (
                <DraftRow
                  key={d.id}
                  draft={d}
                  forumName={forums.find(f => f.id === d.forum_id)?.name}
                  onResume={() => resume(d)}
                  onDelete={() => remove(d)}
                />
              ))}
            </ul>
          )}
        </div>
      </div>

      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}

function DraftRow({ draft, forumName, onResume, onDelete }: {
  draft: Draft
  forumName: string | undefined
  onResume: () => void
  onDelete: () => void
}) {
  const { t } = useTranslation('forum')
  const isReply = !!draft.topic_id
  const { data: topicData } = useQuery({
    queryKey: ['forum-topic-brief', draft.topic_id],
    queryFn: () => forumApi.getTopic(draft.topic_id as string),
    enabled: isReply,
  })

  const title = isReply
    ? topicData?.topic.title
    : draft.title || t('untitled_draft', { defaultValue: 'Brouillon sans titre' })
  const context = isReply
    ? t('reply_draft_context', { defaultValue: 'Réponse dans « {{topic}} »', topic: topicData?.topic.title ?? '…' })
    : t('new_topic_draft_context', { defaultValue: 'Nouveau sujet dans {{forum}}', forum: forumName ?? '…' })

  return (
    <li className="flex items-start gap-3 px-4 py-3">
      <FileText size={18} className="text-text-tertiary mt-0.5 shrink-0" />
      <div className="min-w-0 flex-1">
        <div className="text-xs text-text-tertiary truncate">{context}</div>
        <div className="font-medium text-text-primary truncate">{title}</div>
        {draft.body_md.trim() && (
          <div className="text-sm text-text-secondary truncate">{excerptOf(draft.body_md)}</div>
        )}
        <div className="text-[11px] text-text-tertiary mt-0.5">{timeAgo(draft.updated_at)}</div>
      </div>
      <div className="flex items-center gap-1.5 shrink-0">
        <Button variant="ghost" size="sm" onClick={onResume}>{t('resume_draft', { defaultValue: 'Reprendre' })}</Button>
        <Button variant="ghost" size="sm" icon={<Trash2 size={14} />} onClick={onDelete}>{t('delete_draft', { defaultValue: 'Supprimer' })}</Button>
      </div>
    </li>
  )
}
