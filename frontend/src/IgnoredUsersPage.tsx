import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { UserX } from 'lucide-react'
import { Spinner, Button } from '@ui'
import { forumApi } from './api'
import { useResolveUsers } from './users'
import { AuthorName, AuthorAvatar } from './Author'

/** `/forum/ignored` — the caller's own ignore list (phpBB "foes"/zebra). A
 *  purely client-side display preference: unignoring here only changes what
 *  folds in the topic view, nothing on the server-side post listing. */
export default function IgnoredUsersPage() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()

  const { data: ignored = [], isLoading } = useQuery({
    queryKey: ['forum-ignored'],
    queryFn: forumApi.listIgnored,
  })
  useResolveUsers(ignored)

  const unignore = async (uid: string) => {
    await forumApi.unignoreUser(uid)
    qc.invalidateQueries({ queryKey: ['forum-ignored'] })
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-lg font-semibold text-text-primary">{t('ignored_users', { defaultValue: 'Membres ignorés' })}</h1>
          <span className="text-xs text-text-tertiary">{t('ignored_count', { defaultValue: '{{count}} ignorés', count: ignored.length })}</span>
        </div>

        <div className="rounded-xl border border-border overflow-hidden bg-surface-0">
          {isLoading ? (
            <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
          ) : ignored.length === 0 ? (
            <div className="px-4 py-16 text-center text-text-secondary">
              {t('no_ignored_users', { defaultValue: "Vous n'ignorez personne." })}
            </div>
          ) : (
            <ul className="divide-y divide-border">
              {ignored.map(uid => (
                <li key={uid} className="flex items-center gap-3 px-4 py-2.5">
                  <AuthorAvatar id={uid} size={36} />
                  <div className="min-w-0 flex-1 font-medium text-text-primary truncate">
                    <AuthorName id={uid} />
                  </div>
                  <Button variant="ghost" size="sm" icon={<UserX size={14} />} onClick={() => unignore(uid)}>
                    {t('unignore_user', { defaultValue: 'Ne plus ignorer' })}
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  )
}
