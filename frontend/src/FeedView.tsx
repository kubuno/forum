import { useMemo } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { MessageSquare, Eye, CheckCircle2, Clock } from 'lucide-react'
import { Spinner } from '@ui'
import { forumApi, type Tag, type Topic } from './api'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo } from './helpers'

const TITLES: Record<string, string> = {
  recent: 'feed_recent', unanswered: 'feed_unanswered', popular: 'feed_popular',
  unread: 'feed_unread', mine: 'feed_mine', bookmarks: 'feed_bookmarks',
}

/** Cross-forum discovery list (recent / unanswered / popular / unread / mine / bookmarks). */
export default function FeedView() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const { kind = 'recent' } = useParams()

  const { data, isLoading } = useQuery({
    queryKey: ['forum-feed', kind],
    queryFn: async (): Promise<{ topics: Topic[]; tags: Record<string, Tag[]> }> => {
      if (kind === 'bookmarks') {
        const topics = await forumApi.listBookmarks()
        return { topics, tags: {} }
      }
      return forumApi.feed(kind, { limit: 50 })
    },
  })

  const topics = useMemo(() => data?.topics ?? [], [data])
  const tags = data?.tags ?? {}
  useResolveUsers(topics.map(tp => tp.last_post_user_id ?? tp.author_id))

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <h1 className="text-xl font-semibold text-text-primary mb-4">{t(TITLES[kind] ?? 'feed_recent')}</h1>
        {isLoading ? (
          <div className="py-16 flex justify-center"><Spinner size="lg" /></div>
        ) : topics.length === 0 ? (
          <p className="text-text-tertiary text-sm py-10 text-center">{t('feed_empty')}</p>
        ) : (
          <div className="rounded-xl border border-border overflow-hidden divide-y divide-border">
            {topics.map(tp => (
              <button key={tp.id} onClick={() => navigate(`/forum/topics/${tp.id}`)}
                className="w-full text-left px-4 py-3 hover:bg-surface-1 flex items-start gap-3">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    {tp.is_solved && <CheckCircle2 size={14} className="text-success shrink-0" />}
                    {tp.prefix && <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-primary-light text-primary">{tp.prefix}</span>}
                    <span className="text-sm font-medium text-text-primary truncate">{tp.title}</span>
                    {(tags[tp.id] ?? []).map(tag => (
                      <span key={tag.id} className="px-1.5 py-0.5 rounded-full text-[10px] font-medium" style={{ backgroundColor: tag.color + '22', color: tag.color }}>{tag.name}</span>
                    ))}
                  </div>
                  <div className="flex items-center gap-3 mt-1 text-xs text-text-tertiary">
                    <span className="flex items-center gap-1"><Clock size={11} />{timeAgo(tp.last_post_at ?? tp.created_at)}</span>
                    <span className="flex items-center gap-1"><MessageSquare size={11} />{tp.reply_count}</span>
                    <span className="flex items-center gap-1"><Eye size={11} />{tp.view_count}</span>
                    <span className="truncate">· <AuthorName id={tp.last_post_user_id ?? tp.author_id} /></span>
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
