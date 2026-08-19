import { useEffect } from 'react'
import { useParams, useLocation } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Home, MessagesSquare, ShieldAlert, Settings, Clock, HelpCircle, Flame, Mail,
  User as UserIcon, Bookmark, Users,
} from 'lucide-react'
import { SidebarNavItem, useAuthStore } from '@kubuno/sdk'
import { forumApi } from './api'
import { setQueryClient } from './nav'
import NotificationsBell from './NotificationsBell'

export default function ForumSidebarBody({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('forum')

  // Expose the query client to non-component callbacks (sidebar "New" menu items).
  const qc = useQueryClient()
  useEffect(() => setQueryClient(qc), [qc])
  const params = useParams()
  const { pathname } = useLocation()
  const me = useAuthStore(s => s.user)
  const isAdmin = me?.role === 'admin'
  const activeForumId = params.id ?? null

  const { data: categories = [] } = useQuery({ queryKey: ['forum-categories'], queryFn: forumApi.listCategories })
  const { data: forums = [] } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })
  const { data: stats } = useQuery({ queryKey: ['forum-stats'], queryFn: forumApi.stats, refetchInterval: 60_000 })

  // Heartbeat keeps the user listed in "who's online".
  useEffect(() => {
    forumApi.heartbeat('/forum').catch(() => {})
    const iv = setInterval(() => forumApi.heartbeat('/forum').catch(() => {}), 60_000)
    return () => clearInterval(iv)
  }, [])

  const feedActive = (k: string) => pathname === `/forum/feed/${k}`
  const feed = (k: string, label: string, icon: React.ReactNode) => (
    <SidebarNavItem label={label} icon={icon} collapsed={collapsed} active={feedActive(k)} to={`/forum/feed/${k}`} />
  )

  return (
    <div className="flex flex-col gap-0.5 px-3 py-2">
      <NotificationsBell collapsed={collapsed} />
      <SidebarNavItem
        label={t('forums')}
        icon={<Home size={18} />}
        collapsed={collapsed}
        active={pathname === '/forum'}
        to="/forum"
      />

      {/* Discover */}
      {!collapsed && <div className="px-2 pt-2 pb-1 text-[11px] font-semibold uppercase tracking-wide text-text-tertiary">{t('discover')}</div>}
      {feed('recent', t('feed_recent'), <Clock size={18} />)}
      {feed('unanswered', t('feed_unanswered'), <HelpCircle size={18} />)}
      {feed('popular', t('feed_popular'), <Flame size={18} />)}
      {feed('unread', t('feed_unread'), <Mail size={18} />)}
      {feed('mine', t('feed_mine'), <UserIcon size={18} />)}
      {feed('bookmarks', t('feed_bookmarks'), <Bookmark size={18} />)}

      {categories.map(cat => {
        const top = forums.filter(f => f.category_id === cat.id && !f.parent_forum_id)
        if (top.length === 0) return null
        return (
          <div key={cat.id} className="mt-1">
            {!collapsed && (
              <div className="px-2 pt-2 pb-1 text-[11px] font-semibold uppercase tracking-wide text-text-tertiary truncate">
                {cat.name}
              </div>
            )}
            {top.map(f => (
              <SidebarNavItem
                key={f.id}
                label={f.name}
                icon={<MessagesSquare size={18} />}
                collapsed={collapsed}
                active={activeForumId === f.id}
                to={`/forum/forums/${f.id}`}
              />
            ))}
          </div>
        )
      })}

      {isAdmin && (
        <div className="mt-2 pt-2 border-t border-border space-y-0.5">
          <SidebarNavItem
            label={t('moderation')}
            icon={<ShieldAlert size={18} />}
            collapsed={collapsed}
            active={pathname === '/forum/moderation'}
            to="/forum/moderation"
          />
          <SidebarNavItem
            label={t('settings')}
            icon={<Settings size={18} />}
            collapsed={collapsed}
            active={pathname === '/forum/settings'}
            to="/forum/settings"
          />
        </div>
      )}

      {!collapsed && stats && (
        <div className="mt-2 pt-2 border-t border-border px-2 text-[11px] text-text-tertiary space-y-0.5">
          <div className="flex items-center gap-1.5"><Users size={12} /> {t('stat_online', { count: stats.online })}</div>
          <div>{t('stat_summary', { topics: stats.topics, posts: stats.posts, members: stats.members })}</div>
        </div>
      )}
    </div>
  )
}
