import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { Trophy, MessageSquare } from 'lucide-react'
import { Spinner } from '@ui'
import { forumApi } from './api'
import { useResolveUsers } from './users'
import { AuthorName, AuthorAvatar } from './Author'

const MEDAL_COLOR: Record<number, string> = {
  1: 'text-[#d4af37]',
  2: 'text-[#a8a8a8]',
  3: 'text-[#b08d57]',
}

/** Top contributors by post count. */
export default function LeaderboardPage() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()

  const { data: top = [], isLoading } = useQuery({
    queryKey: ['forum-leaderboard'],
    queryFn: forumApi.leaderboard,
  })
  useResolveUsers(top.map(e => e.user_id))

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center gap-2 mb-4">
          <Trophy size={18} className="text-text-secondary" />
          <h1 className="text-lg font-semibold text-text-primary">{t('leaderboard', { defaultValue: 'Leaderboard' })}</h1>
        </div>

        <div className="rounded-xl border border-border overflow-hidden bg-surface-0">
          {isLoading && top.length === 0 ? (
            <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
          ) : top.length === 0 ? (
            <div className="px-4 py-16 text-center text-text-secondary">{t('no_leaderboard', { defaultValue: 'No contributors yet.' })}</div>
          ) : (
            <ul className="divide-y divide-border">
              {top.map((entry, i) => {
                const rank = i + 1
                return (
                  <li key={entry.user_id}>
                    <button onClick={() => navigate(`/forum/profiles/${entry.user_id}`)} className="w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-surface-1">
                      <span className={`w-6 text-sm font-semibold text-right shrink-0 ${MEDAL_COLOR[rank] ?? 'text-text-tertiary'}`}>
                        {rank}
                      </span>
                      <AuthorAvatar id={entry.user_id} size={36} />
                      <div className="min-w-0 flex-1">
                        <div className="font-medium text-text-primary truncate"><AuthorName id={entry.user_id} /></div>
                      </div>
                      <span className="flex items-center gap-1 text-xs text-text-tertiary shrink-0">
                        <MessageSquare size={12} />{t('post_count', { count: entry.post_count })}
                      </span>
                    </button>
                  </li>
                )
              })}
            </ul>
          )}
        </div>
      </div>
    </div>
  )
}
