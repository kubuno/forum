import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useConfirm, useAuthStore } from '@kubuno/sdk'
import { ShieldAlert, ExternalLink, Clock, MessageSquarePlus, History, Ban as BanIcon, Flag, Trash2, RotateCcw, Globe, Mail } from 'lucide-react'
import { Spinner, Button, Badge, ConfirmDialog, Input } from '@ui'
import { forumApi, type Report, type PendingPost, type ModLogEntry, type Ban, type IpBan, type EmailBan, type Topic } from './api'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo, shortDateTime } from './helpers'

type Tab = 'reports' | 'queue' | 'log' | 'bans' | 'trash'

// The moderation page gathers everything a moderator acts on. Reports and the
// approval queue are the daily work; the log and the ban list — until now with
// no interface at all despite a complete backend — are the record and the roster.

export default function ModerationPanel() {
  const { t } = useTranslation('forum')
  const [tab, setTab] = useState<Tab>('reports')

  const tabs: { key: Tab; label: string; icon: React.ReactNode }[] = [
    { key: 'reports', label: t('reports'), icon: <Flag size={15} /> },
    { key: 'queue',   label: t('pending_queue'), icon: <Clock size={15} /> },
    { key: 'log',     label: t('mod_log', { defaultValue: 'Log' }), icon: <History size={15} /> },
    { key: 'bans',    label: t('bans', { defaultValue: 'Bans' }), icon: <BanIcon size={15} /> },
    { key: 'trash',   label: t('trash', { defaultValue: 'Trash' }), icon: <Trash2 size={15} /> },
  ]

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <h1 className="text-lg font-semibold text-text-primary flex items-center gap-2 mb-4">
          <ShieldAlert size={18} className="text-primary" /> {t('moderation')}
        </h1>

        <div className="flex items-center gap-1 mb-4 flex-wrap">
          {tabs.map(tb => (
            <button
              key={tb.key}
              onClick={() => setTab(tb.key)}
              className={`h-8 px-3 rounded-lg text-sm inline-flex items-center gap-1.5 ${tab === tb.key ? 'bg-primary text-white font-medium' : 'text-text-secondary hover:bg-surface-1'}`}
            >
              {tb.icon}{tb.label}
            </button>
          ))}
        </div>

        {tab === 'reports' && <ReportsTab />}
        {tab === 'queue' && <PendingQueue />}
        {tab === 'log' && <ModLogTab />}
        {tab === 'bans' && <BansTab />}
        {tab === 'trash' && <TrashTab />}
      </div>
    </div>
  )
}

function ReportsTab() {
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

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  if (reports.length === 0) return <div className="py-16 text-center text-text-secondary">{t('no_reports')}</div>

  return (
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
  )
}

function PendingQueue() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const navigate = useNavigate()

  const { data, isLoading } = useQuery({
    queryKey: ['forum-pending'],
    // A moderator of no forum gets a 403 here; that is not an error worth retrying.
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

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  if (pending.length === 0) return <div className="py-16 text-center text-text-secondary">{t('pending_empty', { defaultValue: 'Nothing is waiting for approval.' })}</div>

  return (
    <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
      {pending.map(p => (
        <li key={p.id} className="px-4 py-3">
          <div className="flex items-start gap-3">
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1 flex-wrap">
                {p.is_first_post && (
                  <Badge variant="primary" size="sm"><MessageSquarePlus size={12} className="inline mr-1" />{t('new_topic')}</Badge>
                )}
                <span className="text-xs text-text-tertiary truncate">{p.forum_name} · {p.topic_title}</span>
              </div>
              <div className="text-xs text-text-tertiary mb-1">{t('by')} <AuthorName id={p.author_id} /> · {timeAgo(p.created_at)}</div>
              <p className="text-sm text-text-primary break-words whitespace-pre-wrap line-clamp-6">{p.body_md}</p>
            </div>
            {!p.is_first_post && (
              <button onClick={() => navigate(`/forum/topics/${p.topic_id}`)} title={t('topic')} className="p-1.5 rounded hover:bg-surface-1 text-text-secondary shrink-0">
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
  )
}

function ModLogTab() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const { data: log = [], isLoading } = useQuery({ queryKey: ['forum-modlog'], queryFn: forumApi.modLog })
  useResolveUsers([...log.map(e => e.moderator_id), ...log.map(e => e.target_user_id).filter(Boolean) as string[]])

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  if (log.length === 0) return <div className="py-16 text-center text-text-secondary">{t('no_log', { defaultValue: 'Nothing logged yet.' })}</div>

  const link = (e: ModLogEntry) => e.topic_id ? () => navigate(`/forum/topics/${e.topic_id}`) : undefined

  return (
    <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
      {log.map(e => (
        <li key={e.id} className="px-4 py-2.5 flex items-start gap-2 text-sm">
          <History size={14} className="text-text-tertiary mt-0.5 shrink-0" />
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-1.5 flex-wrap">
              <span className="font-medium text-text-primary"><AuthorName id={e.moderator_id} /></span>
              <Badge variant="neutral" size="sm">{t(`modlog_${e.action}`, { defaultValue: e.action.replace(/_/g, ' ') })}</Badge>
              {e.target_user_id && <span className="text-text-secondary text-xs">→ <AuthorName id={e.target_user_id} /></span>}
              {link(e) && (
                <button onClick={link(e)} className="text-primary hover:underline text-xs inline-flex items-center gap-0.5">
                  {t('topic')} <ExternalLink size={11} />
                </button>
              )}
            </div>
            {e.details && <div className="text-xs text-text-tertiary truncate">{e.details}</div>}
          </div>
          <span className="text-xs text-text-tertiary shrink-0" title={shortDateTime(e.created_at)}>{timeAgo(e.created_at)}</span>
        </li>
      ))}
    </ul>
  )
}

function BansTab() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { data: bans = [], isLoading } = useQuery({ queryKey: ['forum-bans'], queryFn: forumApi.listBans })
  useResolveUsers([...bans.map(b => b.user_id), ...bans.map(b => b.banned_by)])

  const unban = async (b: Ban) => {
    await forumApi.unbanUser(b.user_id)
    qc.invalidateQueries({ queryKey: ['forum-bans'] })
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        {isLoading ? (
          <div className="py-10 flex justify-center"><Spinner /></div>
        ) : bans.length === 0 ? (
          <div className="py-16 text-center text-text-secondary">{t('no_bans', { defaultValue: 'No one is banned.' })}</div>
        ) : (
          <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
            {bans.map(b => (
              <li key={b.user_id} className="px-4 py-3 flex items-start gap-3">
                <div className="min-w-0 flex-1">
                  <div className="font-medium text-text-primary truncate"><AuthorName id={b.user_id} /></div>
                  {b.reason && <div className="text-xs text-text-secondary break-words">{b.reason}</div>}
                  <div className="text-xs text-text-tertiary mt-0.5">
                    {t('banned_by', { defaultValue: 'by' })} <AuthorName id={b.banned_by} /> · {timeAgo(b.created_at)}
                    {' · '}
                    {b.until ? t('ban_until', { defaultValue: 'until {{date}}', date: shortDateTime(b.until) }) : t('ban_permanent', { defaultValue: 'permanent' })}
                  </div>
                </div>
                <Button variant="ghost" size="sm" onClick={() => unban(b)}>{t('unban', { defaultValue: 'Lift' })}</Button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <IpBansSection />
      <EmailBansSection />
    </div>
  )
}

// IP bans (phpBB-style, exact match — see services::ban_registry on the backend).
function IpBansSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const [value, setValue] = useState('')
  const [reason, setReason] = useState('')
  const { data: bans = [], isLoading } = useQuery({ queryKey: ['forum-ip-bans'], queryFn: forumApi.listIpBans })

  const add = async () => {
    const v = value.trim()
    if (!v) return
    await forumApi.banIp(v, reason.trim() || undefined)
    setValue('')
    setReason('')
    qc.invalidateQueries({ queryKey: ['forum-ip-bans'] })
  }

  const remove = async (b: IpBan) => {
    await forumApi.unbanIp(b.id)
    qc.invalidateQueries({ queryKey: ['forum-ip-bans'] })
  }

  return (
    <div>
      <h2 className="text-sm font-semibold text-text-primary mb-2 flex items-center gap-1.5">
        <Globe size={14} className="text-text-secondary" /> {t('ip_bans', { defaultValue: 'Bans par IP' })}
      </h2>
      <div className="flex items-end gap-2 mb-3">
        <Input value={value} onChange={e => setValue(e.target.value)} placeholder={t('ip_ban_placeholder', { defaultValue: 'Adresse IP (ex. 203.0.113.4)' })} />
        <Input value={reason} onChange={e => setReason(e.target.value)} placeholder={t('ban_reason_placeholder', { defaultValue: 'Motif (facultatif)' })} />
        <Button variant="primary" size="sm" onClick={add}>{t('add', { defaultValue: 'Ajouter' })}</Button>
      </div>
      {isLoading ? (
        <div className="py-6 flex justify-center"><Spinner /></div>
      ) : bans.length === 0 ? (
        <div className="py-6 text-center text-sm text-text-secondary">{t('no_ip_bans', { defaultValue: 'Aucune IP bannie.' })}</div>
      ) : (
        <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
          {bans.map(b => (
            <li key={b.id} className="px-4 py-2.5 flex items-start gap-3">
              <div className="min-w-0 flex-1">
                <div className="font-medium text-text-primary truncate">{b.value}</div>
                {b.reason && <div className="text-xs text-text-secondary break-words">{b.reason}</div>}
                <div className="text-xs text-text-tertiary mt-0.5">
                  {timeAgo(b.created_at)}
                  {' · '}
                  {b.until ? t('ban_until', { defaultValue: 'until {{date}}', date: shortDateTime(b.until) }) : t('ban_permanent', { defaultValue: 'permanent' })}
                </div>
              </div>
              <Button variant="ghost" size="sm" onClick={() => remove(b)}>{t('unban', { defaultValue: 'Lift' })}</Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

// Email bans (phpBB-style, exact match — email is normalized to lowercase server-side).
function EmailBansSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const [email, setEmail] = useState('')
  const [reason, setReason] = useState('')
  const { data: bans = [], isLoading } = useQuery({ queryKey: ['forum-email-bans'], queryFn: forumApi.listEmailBans })

  const add = async () => {
    const v = email.trim()
    if (!v) return
    await forumApi.banEmail(v, reason.trim() || undefined)
    setEmail('')
    setReason('')
    qc.invalidateQueries({ queryKey: ['forum-email-bans'] })
  }

  const remove = async (b: EmailBan) => {
    await forumApi.unbanEmail(b.id)
    qc.invalidateQueries({ queryKey: ['forum-email-bans'] })
  }

  return (
    <div>
      <h2 className="text-sm font-semibold text-text-primary mb-2 flex items-center gap-1.5">
        <Mail size={14} className="text-text-secondary" /> {t('email_bans', { defaultValue: 'Bans par e-mail' })}
      </h2>
      <div className="flex items-end gap-2 mb-3">
        <Input value={email} onChange={e => setEmail(e.target.value)} placeholder={t('email_ban_placeholder', { defaultValue: 'Adresse e-mail' })} />
        <Input value={reason} onChange={e => setReason(e.target.value)} placeholder={t('ban_reason_placeholder', { defaultValue: 'Motif (facultatif)' })} />
        <Button variant="primary" size="sm" onClick={add}>{t('add', { defaultValue: 'Ajouter' })}</Button>
      </div>
      {isLoading ? (
        <div className="py-6 flex justify-center"><Spinner /></div>
      ) : bans.length === 0 ? (
        <div className="py-6 text-center text-sm text-text-secondary">{t('no_email_bans', { defaultValue: 'Aucun e-mail banni.' })}</div>
      ) : (
        <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
          {bans.map(b => (
            <li key={b.id} className="px-4 py-2.5 flex items-start gap-3">
              <div className="min-w-0 flex-1">
                <div className="font-medium text-text-primary truncate">{b.email}</div>
                {b.reason && <div className="text-xs text-text-secondary break-words">{b.reason}</div>}
                <div className="text-xs text-text-tertiary mt-0.5">
                  {timeAgo(b.created_at)}
                  {' · '}
                  {b.until ? t('ban_until', { defaultValue: 'until {{date}}', date: shortDateTime(b.until) }) : t('ban_permanent', { defaultValue: 'permanent' })}
                </div>
              </div>
              <Button variant="ghost" size="sm" onClick={() => remove(b)}>{t('unban', { defaultValue: 'Lift' })}</Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

function TrashTab() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const navigate = useNavigate()
  const me = useAuthStore(s => s.user)
  const isAdmin = me?.role === 'admin'
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  const { data: topics = [], isLoading } = useQuery({ queryKey: ['forum-trash'], queryFn: forumApi.listTrash })
  const { data: forums = [] } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })
  useResolveUsers(topics.map(topic => topic.deleted_by).filter(Boolean) as string[])

  const forumName = (forumId: string) => forums.find(f => f.id === forumId)?.name ?? forumId

  const restore = async (topic: Topic) => {
    await forumApi.restoreTopic(topic.id)
    qc.invalidateQueries({ queryKey: ['forum-trash'] })
    qc.invalidateQueries({ queryKey: ['forum-topics'] })
    qc.invalidateQueries({ queryKey: ['forum-forums'] })
    qc.invalidateQueries({ queryKey: ['forum-modlog'] })
  }

  const purge = async (topic: Topic) => {
    const ok = await confirm({
      title: t('purge_topic', { defaultValue: 'Delete permanently' }),
      message: t('confirm_purge_topic', { defaultValue: 'This will permanently erase "{{title}}" and all of its posts. This cannot be undone.', title: topic.title }),
      confirmLabel: t('delete', { defaultValue: 'Delete' }),
      variant: 'danger',
    })
    if (!ok) return
    await forumApi.purgeTopic(topic.id)
    qc.invalidateQueries({ queryKey: ['forum-trash'] })
    qc.invalidateQueries({ queryKey: ['forum-modlog'] })
  }

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  if (topics.length === 0) return <div className="py-16 text-center text-text-secondary">{t('trash_empty', { defaultValue: 'The trash is empty.' })}</div>

  return (
    <>
      <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
        {topics.map(topic => (
          <li key={topic.id} className="px-4 py-3 flex items-start gap-3">
            <div className="min-w-0 flex-1">
              <div className="text-xs text-text-tertiary truncate mb-1">{forumName(topic.forum_id)}</div>
              <div className="font-medium text-text-primary truncate">{topic.title}</div>
              <div className="text-xs text-text-tertiary mt-0.5">
                {t('deleted_by', { defaultValue: 'deleted by' })}{' '}
                {topic.deleted_by ? <AuthorName id={topic.deleted_by} /> : t('unknown_author', { defaultValue: 'unknown' })}
                {' · '}
                <span title={shortDateTime(topic.deleted_at)}>{timeAgo(topic.deleted_at)}</span>
              </div>
            </div>
            <button onClick={() => navigate(`/forum/topics/${topic.id}`)} title={t('topic')} className="p-1.5 rounded hover:bg-surface-1 text-text-secondary shrink-0">
              <ExternalLink size={15} />
            </button>
            <div className="flex flex-col gap-1.5 shrink-0">
              <Button variant="ghost" size="sm" onClick={() => restore(topic)}>
                <RotateCcw size={13} className="mr-1" />{t('restore', { defaultValue: 'Restore' })}
              </Button>
              {isAdmin && (
                <Button variant="danger" size="sm" onClick={() => purge(topic)}>
                  <Trash2 size={13} className="mr-1" />{t('purge', { defaultValue: 'Delete forever' })}
                </Button>
              )}
            </div>
          </li>
        ))}
      </ul>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </>
  )
}
