import { useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Shield } from 'lucide-react'
import { Spinner } from '@ui'
import { forumApi } from './api'
import { useResolveUsers } from './users'
import { AuthorName, AuthorAvatar } from './Author'

/** "The team" page. Lists per-forum moderators, grouped by forum.
 *
 * The core does not expose the platform's admin roster to a module, so this
 * deliberately shows the moderation team only — never platform admins — and
 * only for forums the backend already filtered as visible to the caller. */
export default function TeamPage() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()

  const { data: moderators = [], isLoading } = useQuery({
    queryKey: ['forum-team'],
    queryFn: forumApi.team,
  })
  useResolveUsers(moderators.map(m => m.user_id))

  const groups = useMemo(() => {
    const byForum = new Map<string, { forumId: string; forumName: string; userIds: string[] }>()
    for (const m of moderators) {
      const g = byForum.get(m.forum_id)
      if (g) g.userIds.push(m.user_id)
      else byForum.set(m.forum_id, { forumId: m.forum_id, forumName: m.forum_name, userIds: [m.user_id] })
    }
    return Array.from(byForum.values())
  }, [moderators])

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center gap-2 mb-4">
          <Shield size={18} className="text-text-secondary" />
          <h1 className="text-lg font-semibold text-text-primary">{t('team', { defaultValue: 'Équipe de modération' })}</h1>
        </div>

        {isLoading ? (
          <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
        ) : groups.length === 0 ? (
          <div className="rounded-xl border border-border bg-surface-0 px-4 py-16 text-center text-text-secondary">
            {t('team_empty', { defaultValue: "Aucun forum n'a de modérateur pour le moment." })}
          </div>
        ) : (
          <div className="space-y-4">
            {groups.map(g => (
              <div key={g.forumId} className="rounded-xl border border-border overflow-hidden bg-surface-0">
                <div className="px-4 py-2.5 border-b border-border text-sm font-semibold text-text-primary">
                  {g.forumName}
                </div>
                <ul className="divide-y divide-border">
                  {g.userIds.map(uid => (
                    <li key={uid}>
                      <button
                        onClick={() => navigate(`/forum/profiles/${uid}`)}
                        className="w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-surface-1"
                      >
                        <AuthorAvatar id={uid} size={36} />
                        <div className="min-w-0 flex-1 font-medium text-text-primary truncate">
                          <AuthorName id={uid} />
                        </div>
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
