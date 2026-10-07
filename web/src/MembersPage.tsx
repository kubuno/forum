import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, keepPreviousData } from '@tanstack/react-query'
import { MessageSquare, Clock } from 'lucide-react'
import { Spinner } from '@ui'
import { forumApi } from './api'
import { useResolveUsers } from './users'
import { AuthorName, AuthorAvatar } from './Author'
import { timeAgo } from './helpers'
import Pagination from './Pagination'

const MEMBERS_PER_PAGE = 30
type Sort = 'posts' | 'recent' | 'active'

export default function MembersPage() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const [sort, setSort] = useState<Sort>('posts')
  const [page, setPage] = useState(1)

  const { data, isLoading } = useQuery({
    queryKey: ['forum-members', sort, page],
    queryFn: () => forumApi.members({ sort, limit: MEMBERS_PER_PAGE, offset: (page - 1) * MEMBERS_PER_PAGE }),
    placeholderData: keepPreviousData,
  })
  const members = data?.members ?? []
  const total = data?.total ?? 0
  useResolveUsers(members.map(m => m.user_id))

  const tabs: { key: Sort; label: string }[] = [
    { key: 'posts',  label: t('sort_posts',  { defaultValue: 'Most posts' }) },
    { key: 'recent', label: t('sort_recent', { defaultValue: 'Newest' }) },
    { key: 'active', label: t('sort_active', { defaultValue: 'Recently active' }) },
  ]
  const setTab = (s: Sort) => { setSort(s); setPage(1) }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-lg font-semibold text-text-primary">{t('members', { defaultValue: 'Members' })}</h1>
          <span className="text-xs text-text-tertiary">{t('member_count', { defaultValue: '{{count}} members', count: total })}</span>
        </div>

        <div className="flex items-center gap-1 mb-3">
          {tabs.map(tab => (
            <button
              key={tab.key}
              onClick={() => setTab(tab.key)}
              className={`h-8 px-3 rounded-lg text-sm ${sort === tab.key ? 'bg-primary text-white font-medium' : 'text-text-secondary hover:bg-surface-1'}`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="rounded-xl border border-border overflow-hidden bg-surface-0">
          {isLoading && members.length === 0 ? (
            <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
          ) : members.length === 0 ? (
            <div className="px-4 py-16 text-center text-text-secondary">{t('no_members', { defaultValue: 'No members yet.' })}</div>
          ) : (
            <ul className="divide-y divide-border">
              {members.map(m => (
                <li key={m.user_id}>
                  <button onClick={() => navigate(`/forum/profiles/${m.user_id}`)} className="w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-surface-1">
                    <AuthorAvatar id={m.user_id} size={36} />
                    <div className="min-w-0 flex-1">
                      <div className="font-medium text-text-primary truncate"><AuthorName id={m.user_id} /></div>
                      {(m.rank_badge || m.rank_title) && (
                        <div className="text-xs text-text-tertiary truncate flex items-center gap-1">
                          {m.rank_badge && <span>{m.rank_badge}</span>}{m.rank_title}
                        </div>
                      )}
                    </div>
                    <div className="flex flex-col items-end text-xs text-text-tertiary shrink-0">
                      <span className="flex items-center gap-1"><MessageSquare size={12} />{m.post_count}</span>
                      {m.last_seen_at && <span className="flex items-center gap-1"><Clock size={12} />{timeAgo(m.last_seen_at)}</span>}
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <Pagination page={page} pageSize={MEMBERS_PER_PAGE} total={total} onPage={setPage} />
      </div>
    </div>
  )
}
