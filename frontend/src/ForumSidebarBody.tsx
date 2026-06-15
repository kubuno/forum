import { useEffect } from 'react'
import { useNavigate, useParams, useLocation } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Home, MessagesSquare, ShieldAlert, Settings } from 'lucide-react'
import { SidebarNavItem, useAuthStore } from '@kubuno/sdk'
import { forumApi } from './api'
import { setNav } from './nav'

export default function ForumSidebarBody({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()

  // Expose navigation to non-component callbacks (host search bar).
  useEffect(() => setNav(navigate), [navigate])
  const params = useParams()
  const { pathname } = useLocation()
  const me = useAuthStore(s => s.user)
  const isAdmin = me?.role === 'admin'
  const activeForumId = params.id ?? null

  const { data: categories = [] } = useQuery({ queryKey: ['forum-categories'], queryFn: forumApi.listCategories })
  const { data: forums = [] } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })

  return (
    <div className="flex flex-col gap-0.5 px-2 py-2">
      <SidebarNavItem
        label={t('forums')}
        icon={<Home size={18} />}
        collapsed={collapsed}
        active={pathname === '/forum'}
        onClick={() => navigate('/forum')}
      />

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
                onClick={() => navigate(`/forum/forums/${f.id}`)}
              />
            ))}
          </div>
        )
      })}

      {isAdmin && (
        <div className="mt-2 pt-2 border-t border-border">
          <SidebarNavItem
            label={t('moderation')}
            icon={<ShieldAlert size={18} />}
            collapsed={collapsed}
            active={pathname === '/forum/moderation'}
            onClick={() => navigate('/forum/moderation')}
          />
          <SidebarNavItem
            label={t('settings')}
            icon={<Settings size={18} />}
            collapsed={collapsed}
            active={pathname === '/forum/settings'}
            onClick={() => navigate('/forum/settings')}
          />
        </div>
      )}
    </div>
  )
}
