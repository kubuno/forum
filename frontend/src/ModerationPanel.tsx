import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ShieldAlert, ExternalLink, Clock, MessageSquarePlus } from 'lucide-react'
import { Spinner, Button, Badge } from '@ui'
import { forumApi, type Report, type PendingPost } from './api'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo } from './helpers'

// The moderation page holds the two things a moderator acts on: contributions
// waiting for a decision (the approval queue, armed by the instance's
// moderation policy) and messages readers have flagged. The queue comes first —
// it is the one where nothing is visible until someone acts.

function PendingQueue() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const navigate = useNavigate()

  const { data, isLoading } = useQuery({
    queryKey: ['forum-pending'],
    // A moderator of no forum gets a 403 here; that is not an error worth
    // retrying, and the section simply stays out of their way.
    queryFn: () => forumApi.pendingQueue().catch(() => ({ pending: [] as PendingPost[], total: 0 })),
  })
  const pending = data?.pending ?? []
  useResolveUsers(pending.map(p => p.author_id))

  const decide = async (p: PendingPost, approve: boolean) => {
    if (approve) await forumApi.approvePending(p.id)
    else await forumApi.rejectPending(p.id)
    qc.invalidateQueries({ queryKey: ['forum-pending'] })
    qc.invalidateQueries({ queryKey: ['forum-topics'] })
    qc.invalidateQueries({ queryKey: ['forum-forums'] })
  }

  if (isLoading) return <div className="py-6 flex justify-center"><Spinner /></div>
  // Nothing waiting, or moderation is simply off on this instance: say nothing.
  if (pending.length === 0) return null

  return (
    <section className="mb-6">
      <h2 className="text-sm font-semibold text-text-primary flex items-center gap-2 mb-2">
        <Clock size={16} className="text-primary" /> {t('pending_queue')}
        <Badge variant="warning" size="sm">{data?.total ?? pending.length}</Badge>
      </h2>
      <p className="text-xs text-text-secondary mb-2">{t('pending_queue_hint')}</p>
      <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
        {pending.map(p => (
          <li key={p.id} className="px-4 py-3">
            <div className="flex items-start gap-3">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1 flex-wrap">
                  {p.is_first_post && (
                    <Badge variant="info" size="sm">
                      <MessageSquarePlus size={12} className="inline mr-1" />{t('new_topic')}
                    </Badge>
                  )}
                  <span className="text-xs text-text-tertiary truncate">
                    {p.forum_name} · {p.topic_title}
                  </span>
                </div>
                <div className="text-xs text-text-tertiary mb-1">
                  {t('by')} <AuthorName id={p.author_id} /> · {timeAgo(p.created_at)}
                </div>
                <p className="text-sm text-text-primary break-words whitespace-pre-wrap line-clamp-6">{p.body_md}</p>
              </div>
              {/* A held topic has nothing to open yet — only replies point somewhere. */}
              {!p.is_first_post && (
                <button
                  onClick={() => navigate(`/forum/topics/${p.topic_id}`)}
                  title={t('topic')}
                  className="p-1.5 rounded hover:bg-surface-1 text-text-secondary shrink-0"
                >
                  <ExternalLink size={15} />
                </button>
              )}
            </div>
            <div className="flex justify-end gap-2 mt-2">
              <Button variant="ghost" size="sm" onClick={() => decide(p, false)}>{t('reject')}</Button>
              <Button variant="primary" size="sm" onClick={() => decide(p, true)}>{t('approve')}</Button>
            </div>
          </li>
        ))}
      </ul>
    </section>
  )
}

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
        <PendingQueue />

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
