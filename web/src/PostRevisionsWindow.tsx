import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { History } from 'lucide-react'
import { FloatingWindow, Spinner } from '@ui'
import { forumApi, type Post } from './api'
import { AuthorName } from './Author'
import { shortDateTime } from './helpers'
import PostBody from './PostBody'

interface Props {
  post: Post
  onClose: () => void
}

/** Lists the past versions of a post's body, most recent edit first, plus the
 *  message as it reads now. Only ever opened for a post whose `edit_count` is
 *  positive, and only reachable by its author or a moderator — the backend
 *  enforces the same rule regardless. */
export default function PostRevisionsWindow({ post, onClose }: Props) {
  const { t } = useTranslation('forum')
  const { data: revisions, isLoading } = useQuery({
    queryKey: ['forum-post-revisions', post.id],
    queryFn: () => forumApi.getPostRevisions(post.id),
  })

  return (
    <FloatingWindow title={t('edit_history', { defaultValue: 'Historique des modifications' })} icon={<History size={18} />} onClose={onClose} defaultWidth={520} defaultHeight={480}>
      <div className="p-4 flex flex-col gap-3 h-full overflow-auto">
        {isLoading ? (
          <div className="py-6 flex justify-center"><Spinner size="sm" /></div>
        ) : (
          <>
            <RevisionEntry
              label={t('current_version', { defaultValue: 'Version actuelle' })}
              body={post.body_md}
              editedBy={post.edited_by}
              editReason={post.edit_reason}
              createdAt={post.edited_at ?? post.created_at}
            />
            {(revisions ?? []).map(r => (
              <RevisionEntry
                key={r.id}
                label={t('previous_version', { defaultValue: 'Version précédente' })}
                body={r.body_md}
                editedBy={r.edited_by}
                editReason={r.edit_reason}
                createdAt={r.created_at}
              />
            ))}
            {revisions && revisions.length === 0 && (
              <div className="text-sm text-text-tertiary py-4 text-center">
                {t('no_revisions', { defaultValue: 'Aucune version antérieure disponible.' })}
              </div>
            )}
          </>
        )}
      </div>
    </FloatingWindow>
  )
}

function RevisionEntry({
  label, body, editedBy, editReason, createdAt,
}: {
  label: string
  body: string
  editedBy: string | null
  editReason: string | null
  createdAt: string
}) {
  const { t } = useTranslation('forum')
  return (
    <div className="rounded-lg border border-border bg-surface-1 p-3">
      <div className="flex items-center gap-2 text-xs text-text-tertiary mb-2">
        <span className="font-medium text-text-secondary">{label}</span>
        <span>·</span>
        <span>{shortDateTime(createdAt)}</span>
        {editedBy && (
          <>
            <span>·</span>
            <span>{t('edited_by', { defaultValue: 'par' })} <AuthorName id={editedBy} /></span>
          </>
        )}
      </div>
      {editReason && (
        <div className="text-xs italic text-text-tertiary mb-2">{editReason}</div>
      )}
      <PostBody body={body} />
    </div>
  )
}
