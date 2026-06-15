import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ShieldAlert, ExternalLink } from 'lucide-react'
import { Spinner, Button, Badge } from '@ui'
import { forumApi, type Report } from './api'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo } from './helpers'

export default function ModerationPanel() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const navigate = useNavigate()

  const { data: reports = [], isLoading } = useQuery({ queryKey: ['forum-reports', 'open'], queryFn: () => forumApi.listReports('open') })
  useResolveUsers(reports.map(r => r.reporter_id))

  const resolve = async (r: Report, status: 'resolved' | 'rejected') => {
    await forumApi.resolveReport(r.id, status)
    qc.invalidateQueries({ queryKey: ['forum-reports'] })
  }

  const openPost = async (r: Report) => {
    try { const p = await forumApi.getPost(r.post_id); navigate(`/forum/topics/${p.topic_id}`) } catch { /* removed */ }
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <h1 className="text-lg font-semibold text-text-primary flex items-center gap-2 mb-4">
          <ShieldAlert size={18} className="text-primary" /> {t('reports')}
        </h1>
        {isLoading ? (
          <div className="py-10 flex justify-center"><Spinner /></div>
        ) : reports.length === 0 ? (
          <div className="py-16 text-center text-text-secondary">{t('no_reports')}</div>
        ) : (
          <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
            {reports.map(r => (
              <li key={r.id} className="px-4 py-3">
                <div className="flex items-start gap-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <Badge variant="warning" size="sm">{t('status_open')}</Badge>
                      <span className="text-xs text-text-tertiary">{t('by')} <AuthorName id={r.reporter_id} /> · {timeAgo(r.created_at)}</span>
                    </div>
                    <p className="text-sm text-text-primary break-words">{r.reason}</p>
                  </div>
                  <button onClick={() => openPost(r)} title={t('post')} className="p-1.5 rounded hover:bg-surface-1 text-text-secondary shrink-0">
                    <ExternalLink size={15} />
                  </button>
                </div>
                <div className="flex justify-end gap-2 mt-2">
                  <Button variant="ghost" size="sm" onClick={() => resolve(r, 'rejected')}>{t('reject')}</Button>
                  <Button variant="primary" size="sm" onClick={() => resolve(r, 'resolved')}>{t('resolve')}</Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
