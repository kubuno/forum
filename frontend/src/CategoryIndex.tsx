import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { MessagesSquare, Lock, MessageSquare } from 'lucide-react'
import { Spinner } from '@ui'
import { forumApi, type Forum } from './api'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo } from './helpers'

export default function CategoryIndex() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()

  const { data: categories = [], isLoading: lc } = useQuery({ queryKey: ['forum-categories'], queryFn: forumApi.listCategories })
  const { data: forums = [], isLoading: lf } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })

  useResolveUsers(forums.map(f => f.last_post_user_id))

  if (lc || lf) {
    return <div className="h-full flex items-center justify-center"><Spinner size="lg" /></div>
  }

  // Top-level forums grouped by category (sub-forums are shown inside their forum view).
  const topForums = forums.filter(f => !f.parent_forum_id)
  const byCategory = (cid: string) => topForums.filter(f => f.category_id === cid)

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-4xl mx-auto px-4 py-6 space-y-6">
        {categories.length === 0 && (
          <div className="text-center text-text-secondary py-20">{t('no_forums')}</div>
        )}
        {categories.map(cat => (
          <section key={cat.id} className="rounded-xl border border-border overflow-hidden bg-surface-0">
            <header className="px-4 py-2.5 bg-surface-1 border-b border-border">
              <h2 className="text-sm font-semibold text-text-primary">{cat.name}</h2>
              {cat.description && <p className="text-xs text-text-secondary mt-0.5">{cat.description}</p>}
            </header>
            <ul className="divide-y divide-border">
              {byCategory(cat.id).map(f => <ForumRow key={f.id} forum={f} onOpen={() => navigate(`/forum/forums/${f.id}`)} />)}
              {byCategory(cat.id).length === 0 && (
                <li className="px-4 py-4 text-sm text-text-tertiary">{t('no_forums')}</li>
              )}
            </ul>
          </section>
        ))}
      </div>
    </div>
  )

  function ForumRow({ forum: f, onOpen }: { forum: Forum; onOpen: () => void }) {
    return (
      <li>
        <button onClick={onOpen} className="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-surface-1">
          <div className="w-10 h-10 rounded-lg bg-primary-light text-primary flex items-center justify-center shrink-0">
            {f.is_locked ? <Lock size={18} /> : <MessagesSquare size={18} />}
          </div>
          <div className="min-w-0 flex-1">
            <div className="font-medium text-text-primary truncate">{f.name}</div>
            {f.description && <div className="text-xs text-text-secondary truncate">{f.description}</div>}
          </div>
          <div className="hidden sm:flex flex-col items-end text-xs text-text-tertiary shrink-0 w-28">
            <span className="flex items-center gap-1"><MessageSquare size={12} />{t('topic_count', { count: f.topic_count })}</span>
            <span>{t('post_count', { count: f.post_count })}</span>
          </div>
          <div className="hidden md:block text-xs text-text-tertiary shrink-0 w-40 truncate">
            {f.last_post_at ? (
              <>
                <div className="truncate">{timeAgo(f.last_post_at)}</div>
                <div className="truncate">{t('by')} <AuthorName id={f.last_post_user_id} /></div>
              </>
            ) : <span>{t('no_post_yet')}</span>}
          </div>
        </button>
      </li>
    )
  }
}
