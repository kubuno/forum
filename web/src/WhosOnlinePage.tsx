import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Users } from 'lucide-react'
import { Spinner } from '@ui'
import { forumApi } from './api'
import { useResolveUsers } from './users'
import { AuthorName, AuthorAvatar } from './Author'

/** Detailed "who's online" page: every active user, with their current
 * location when the caller is allowed to see it (see `online_detailed` on
 * the backend — a restricted forum's topic never leaks through here). */
export default function WhosOnlinePage() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()

  const { data: users = [], isLoading } = useQuery({
    queryKey: ['forum-online-detailed'],
    queryFn: forumApi.whosOnlineDetailed,
    refetchInterval: 30_000,
  })
  useResolveUsers(users.map(u => u.user_id))

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-lg font-semibold text-text-primary">{t('whos_online', { defaultValue: "Who's online" })}</h1>
          <span className="text-xs text-text-tertiary">{t('stat_online', { count: users.length })}</span>
        </div>

        <div className="rounded-xl border border-border overflow-hidden bg-surface-0">
          {isLoading && users.length === 0 ? (
            <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
          ) : users.length === 0 ? (
            <div className="px-4 py-16 text-center text-text-secondary">
              <Users size={28} className="mx-auto mb-2 text-text-tertiary" />
              {t('no_one_online', { defaultValue: 'No one online right now.' })}
            </div>
          ) : (
            <ul className="divide-y divide-border">
              {users.map(u => (
                <li key={u.user_id}>
                  <button onClick={() => navigate(`/forum/profiles/${u.user_id}`)} className="w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-surface-1">
                    <span className="relative shrink-0">
                      <AuthorAvatar id={u.user_id} size={36} />
                      <span className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-success border-2 border-surface-0" />
                    </span>
                    <div className="min-w-0 flex-1">
                      <div className="font-medium text-text-primary truncate"><AuthorName id={u.user_id} /></div>
                      <div className="text-xs text-text-tertiary truncate">
                        {u.path ?? t('online_no_detail', { defaultValue: 'Online' })}
                      </div>
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  )
}
