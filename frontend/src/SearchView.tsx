import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Search } from 'lucide-react'
import { Input, Spinner } from '@ui'
import { forumApi } from './api'
import { useForumStore } from './store'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo } from './helpers'

export default function SearchView() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const storeQuery = useForumStore(s => s.searchQuery)
  const setStoreQuery = useForumStore(s => s.setSearchQuery)
  const [local, setLocal] = useState(storeQuery)
  const query = storeQuery.trim()

  const { data: results = [], isFetching } = useQuery({
    queryKey: ['forum-search', query],
    queryFn: () => forumApi.search(query),
    enabled: query.length > 0,
  })
  useResolveUsers(results.map(r => r.author_id))

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <form onSubmit={(e) => { e.preventDefault(); setStoreQuery(local) }} className="mb-4">
          <Input value={local} onChange={(e) => setLocal(e.target.value)} autoFocus
            leftIcon={<Search size={16} />} placeholder={t('search_ph')} />
        </form>
        <h1 className="text-sm font-semibold text-text-secondary mb-3">{t('search_results')}</h1>
        {isFetching ? (
          <div className="py-10 flex justify-center"><Spinner /></div>
        ) : results.length === 0 ? (
          <div className="py-10 text-center text-text-secondary">{t('no_results')}</div>
        ) : (
          <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
            {results.map(r => (
              <li key={r.post_id}>
                <button onClick={() => navigate(`/forum/topics/${r.topic_id}`)} className="w-full text-left px-4 py-3 hover:bg-surface-1">
                  <div className="font-medium text-text-primary truncate">{r.topic_title}</div>
                  <div className="text-sm text-text-secondary line-clamp-2">{r.snippet}</div>
                  <div className="text-xs text-text-tertiary mt-1">{t('by')} <AuthorName id={r.author_id} /> · {timeAgo(r.created_at)}</div>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
