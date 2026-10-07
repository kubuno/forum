import type { MenuItem } from '@ui'
import { MessageSquarePlus, FolderPlus, MessagesSquare } from 'lucide-react'
import { i18n, prompt, useAuthStore, navigate } from '@kubuno/sdk'
import { forumApi } from './api'
import { getQueryClient } from './nav'

/**
 * Items for the sidebar "New" button (`shell.new-actions` extension point).
 * Built when the menu opens — fresh labels, role and route, no hooks.
 */
export function newActionItems(): MenuItem[] {
  const t = (key: string) => i18n.t(`forum:${key}`)

  const me = useAuthStore.getState().user
  const isAdmin = me?.role === 'admin'

  // The :id segment only refers to a forum on the forum-view route.
  const match = /^\/forum\/forums\/([^/?#]+)/.exec(window.location.pathname)
  const activeForumId = match ? match[1] : null

  const newTopic = () => {
    if (activeForumId) navigate(`/forum/forums/${activeForumId}?new=1`)
    else navigate('/forum')
  }

  const newCategory = async () => {
    const name = await prompt({ title: t('new_category'), placeholder: t('name'), confirmLabel: t('create') })
    if (!name?.trim()) return
    await forumApi.createCategory({ name: name.trim() })
    getQueryClient()?.invalidateQueries({ queryKey: ['forum-categories'] })
  }

  const items: MenuItem[] = [
    {
      type: 'action',
      label: t('new_topic'),
      icon: <MessageSquarePlus size={16} />,
      onClick: newTopic,
    },
  ]

  if (isAdmin) {
    items.push(
      {
        type: 'action',
        label: t('new_category'),
        icon: <FolderPlus size={16} />,
        onClick: () => { void newCategory() },
      },
      {
        type: 'action',
        label: t('new_forum'),
        icon: <MessagesSquare size={16} />,
        onClick: () => navigate('/forum/settings'),
      },
    )
  }

  return items
}
