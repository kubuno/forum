import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { MessageSquarePlus, FolderPlus, MessagesSquare } from 'lucide-react'
import { useNavigate, useParams, useLocation } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { prompt, useAuthStore } from '@kubuno/sdk'
import { forumApi } from './api'

const ITEM_CLASS =
  'flex items-center gap-3 w-full px-3 py-2 text-sm text-text-primary ' +
  'hover:bg-surface-1 cursor-pointer outline-none'

export default function ForumCreateMenu() {
  const navigate = useNavigate()
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const params = useParams()
  const { pathname } = useLocation()
  const me = useAuthStore(s => s.user)
  const isAdmin = me?.role === 'admin'

  // The :id param only refers to a forum on the forum-view route.
  const activeForumId = pathname.startsWith('/forum/forums/') ? (params.id ?? null) : null

  const newTopic = () => {
    if (activeForumId) navigate(`/forum/forums/${activeForumId}?new=1`)
    else navigate('/forum')
  }

  const newCategory = async () => {
    const name = await prompt({ title: t('new_category'), placeholder: t('name'), confirmLabel: t('create') })
    if (!name?.trim()) return
    await forumApi.createCategory({ name: name.trim() })
    qc.invalidateQueries({ queryKey: ['forum-categories'] })
  }

  return (
    <>
      <DropdownMenu.Item onSelect={newTopic} className={ITEM_CLASS}>
        <MessageSquarePlus size={16} className="text-text-secondary" />
        {t('new_topic')}
      </DropdownMenu.Item>
      {isAdmin && (
        <>
          <DropdownMenu.Item onSelect={newCategory} className={ITEM_CLASS}>
            <FolderPlus size={16} className="text-text-secondary" />
            {t('new_category')}
          </DropdownMenu.Item>
          <DropdownMenu.Item onSelect={() => navigate('/forum/settings')} className={ITEM_CLASS}>
            <MessagesSquare size={16} className="text-text-secondary" />
            {t('new_forum')}
          </DropdownMenu.Item>
        </>
      )}
    </>
  )
}
