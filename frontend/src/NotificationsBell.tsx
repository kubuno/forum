import { useEffect, useState, useRef } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { Bell } from 'lucide-react'
import { forumApi, type ForumNotification } from './api'
import { useResolveUsers, useUser } from './users'
import { goTo } from './nav'
import { timeAgo } from './helpers'

const KIND_LABEL: Record<string, string> = {
  reply: 'notif_reply', mention: 'notif_mention', reaction: 'notif_reaction',
  solution: 'notif_solution', quote: 'notif_quote', topic: 'notif_topic',
}

/** Notification bell with an unread badge and a dropdown list. Polls every 30s. */
export default function NotificationsBell({ collapsed }: { collapsed?: boolean }) {
  const { t } = useTranslation('forum')
  const [unread, setUnread] = useState(0)
  const [open, setOpen] = useState(false)
  const [items, setItems] = useState<ForumNotification[]>([])
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null)
  const btnRef = useRef<HTMLButtonElement>(null)
  useResolveUsers(items.map(n => n.actor_id).filter(Boolean) as string[])

  const refreshCount = () => forumApi.listNotifications(false).then(r => setUnread(r.unread)).catch(() => {})
  useEffect(() => {
    refreshCount()
    const iv = setInterval(refreshCount, 30_000)
    return () => clearInterval(iv)
  }, [])

  const toggle = () => {
    if (open) { setOpen(false); return }
    const r = btnRef.current?.getBoundingClientRect()
    if (r) setPos({ top: r.bottom + 6, left: r.left })
    forumApi.listNotifications(false).then(r => { setItems(r.notifications); setUnread(r.unread) })
    setOpen(true)
  }

  const onClick = async (n: ForumNotification) => {
    setOpen(false)
    await forumApi.markNotifications(n.id).then(setUnread).catch(() => {})
    if (n.topic_id) goTo(`/forum/topics/${n.topic_id}`)
  }
  const markAll = async () => { await forumApi.markNotifications().then(setUnread); setItems(its => its.map(n => ({ ...n, is_read: true }))) }

  return (
    <>
      <button ref={btnRef} onClick={toggle}
        className={`relative flex items-center gap-3 rounded-lg px-2 py-2 text-sm hover:bg-surface-1 text-text-secondary ${collapsed ? 'justify-center' : ''}`}>
        <Bell size={18} />
        {!collapsed && <span className="flex-1 text-left">{t('notifications')}</span>}
        {unread > 0 && (
          <span className="absolute top-1 left-5 min-w-[16px] h-4 px-1 rounded-full bg-danger text-white text-[10px] font-bold flex items-center justify-center">
            {unread > 99 ? '99+' : unread}
          </span>
        )}
      </button>
      {open && pos && createPortal(
        <>
          <div className="fixed inset-0 z-[9998]" onClick={() => setOpen(false)} />
          <div style={{ position: 'fixed', top: pos.top, left: pos.left, zIndex: 9999 }}
            className="w-80 max-h-[70vh] overflow-auto bg-surface-0 border border-border rounded-xl shadow-xl">
            <div className="flex items-center justify-between px-3 py-2 border-b border-border sticky top-0 bg-surface-0">
              <span className="text-sm font-semibold text-text-primary">{t('notifications')}</span>
              <button onClick={markAll} className="text-xs text-primary hover:underline">{t('notif_mark_all')}</button>
            </div>
            {items.length === 0 ? (
              <p className="px-3 py-6 text-center text-sm text-text-tertiary">{t('notif_empty')}</p>
            ) : items.map(n => <NotifRow key={n.id} n={n} onClick={() => onClick(n)} label={t(KIND_LABEL[n.kind] ?? 'notif_topic')} ago={timeAgo(n.created_at)} />)}
          </div>
        </>,
        document.body,
      )}
    </>
  )
}

function NotifRow({ n, onClick, label, ago }: { n: ForumNotification; onClick: () => void; label: string; ago: string }) {
  const actor = useUser(n.actor_id)
  return (
    <button onClick={onClick}
      className={`w-full text-left px-3 py-2 hover:bg-surface-1 border-b border-border/50 ${n.is_read ? '' : 'bg-primary-light/30'}`}>
      <p className="text-sm text-text-primary">
        <span className="font-medium">{actor?.display_name ?? actor?.username ?? '?'}</span> {label}
        {n.extra && <span className="ml-1">{n.extra}</span>}
      </p>
      <p className="text-xs text-text-tertiary">{ago}</p>
    </button>
  )
}
