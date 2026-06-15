import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import {
  MessagesSquare, Lock, Pin, Megaphone, MessageSquare, Eye, ChevronLeft, Plus, Bell, BellOff,
} from 'lucide-react'
import { Spinner, Button, Badge } from '@ui'
import { forumApi, type Topic } from './api'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo } from './helpers'
import NewTopicWindow from './NewTopicWindow'

export default function ForumView() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const { id: forumId = '' } = useParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const [composing, setComposing] = useState(false)
  const [watching, setWatching] = useState(false)

  // Open the composer when arriving with ?new=1 (from the global "New" button).
  useEffect(() => {
    if (searchParams.get('new') === '1') {
      setComposing(true)
      searchParams.delete('new')
      setSearchParams(searchParams, { replace: true })
    }
  }, [searchParams, setSearchParams])

  const { data: forumData, isLoading: lf } = useQuery({
    queryKey: ['forum-forum', forumId],
    queryFn: () => forumApi.getForum(forumId),
    enabled: !!forumId,
  })
  const { data: topicsData, isLoading: lt } = useQuery({
    queryKey: ['forum-topics', forumId],
    queryFn: () => forumApi.listTopics(forumId),
    enabled: !!forumId,
  })
  const { data: subForums = [] } = useQuery({
    queryKey: ['forum-subforums', forumId],
    queryFn: () => forumApi.listForums().then(all => all.filter(f => f.parent_forum_id === forumId)),
    enabled: !!forumId,
  })
  const { data: readState = [] } = useQuery({
    queryKey: ['forum-readstate', forumId],
    queryFn: () => forumApi.forumReadState(forumId),
    enabled: !!forumId,
  })

  const topics = topicsData?.topics ?? []
  useResolveUsers([...topics.map(t => t.last_post_user_id), ...topics.map(t => t.author_id)])

  const readMap = useMemo(() => {
    const m = new Map<string, string | null>()
    readState.forEach(r => m.set(r.topic_id, r.last_read_post_id))
    return m
  }, [readState])

  const toggleWatch = async () => {
    if (watching) { await forumApi.unsubscribeForum(forumId); setWatching(false) }
    else { await forumApi.subscribeForum(forumId); setWatching(true) }
  }

  if (lf || lt) return <div className="h-full flex items-center justify-center"><Spinner size="lg" /></div>
  if (!forumData) return null
  const { forum, permissions } = forumData

  const isUnread = (tp: Topic) => {
    if (!tp.last_post_id) return false
    if (!readMap.has(tp.id)) return true
    return readMap.get(tp.id) !== tp.last_post_id
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-4xl mx-auto px-4 py-5">
        {/* Header */}
        <div className="flex items-center gap-2 mb-4">
          <button onClick={() => navigate('/forum')} className="p-1.5 rounded hover:bg-surface-1 text-text-secondary" title={t('back')}>
            <ChevronLeft size={18} />
          </button>
          <div className="min-w-0 flex-1">
            <h1 className="text-lg font-semibold text-text-primary truncate flex items-center gap-2">
              {forum.is_locked && <Lock size={15} className="text-text-tertiary" />}
              {forum.name}
            </h1>
            {forum.description && <p className="text-xs text-text-secondary truncate">{forum.description}</p>}
          </div>
          <button onClick={toggleWatch} title={watching ? t('unsubscribe') : t('subscribe')}
            className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary">
            {watching ? <BellOff size={16} /> : <Bell size={16} />}
          </button>
          {permissions.can_post && !forum.is_locked && (
            <Button variant="primary" icon={<Plus size={16} />} onClick={() => setComposing(true)}>{t('new_topic')}</Button>
          )}
        </div>

        {/* Sub-forums */}
        {subForums.length > 0 && (
          <div className="rounded-xl border border-border overflow-hidden bg-surface-0 mb-4">
            <ul className="divide-y divide-border">
              {subForums.map(sf => (
                <li key={sf.id}>
                  <button onClick={() => navigate(`/forum/forums/${sf.id}`)} className="w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-surface-1">
                    <MessagesSquare size={16} className="text-primary shrink-0" />
                    <span className="font-medium text-text-primary truncate flex-1">{sf.name}</span>
                    <span className="text-xs text-text-tertiary">{t('topic_count', { count: sf.topic_count })}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Topics */}
        <div className="rounded-xl border border-border overflow-hidden bg-surface-0">
          {topics.length === 0 ? (
            <div className="px-4 py-16 text-center text-text-secondary">{t('no_topics')}</div>
          ) : (
            <ul className="divide-y divide-border">
              {topics.map(tp => (
                <li key={tp.id}>
                  <button onClick={() => navigate(`/forum/topics/${tp.id}`)} className="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-surface-1">
                    <TypeIcon type={tp.topic_type} unread={isUnread(tp)} locked={tp.is_locked} />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className={`truncate ${isUnread(tp) ? 'font-semibold text-text-primary' : 'font-medium text-text-primary'}`}>{tp.title}</span>
                        {tp.topic_type === 'announcement' && <Badge variant="warning" size="sm">{t('type_announcement')}</Badge>}
                        {tp.topic_type === 'global' && <Badge variant="primary" size="sm">{t('type_global')}</Badge>}
                        {tp.is_locked && <Lock size={12} className="text-text-tertiary shrink-0" />}
                      </div>
                      <div className="text-xs text-text-tertiary truncate">
                        {t('by')} <AuthorName id={tp.author_id} /> · {timeAgo(tp.created_at)}
                      </div>
                    </div>
                    <div className="hidden sm:flex flex-col items-end text-xs text-text-tertiary shrink-0 w-20">
                      <span className="flex items-center gap-1"><MessageSquare size={12} />{tp.reply_count}</span>
                      <span className="flex items-center gap-1"><Eye size={12} />{tp.view_count}</span>
                    </div>
                    <div className="hidden md:block text-xs text-text-tertiary shrink-0 w-36 truncate">
                      {tp.last_post_at && (
                        <>
                          <div className="truncate">{timeAgo(tp.last_post_at)}</div>
                          <div className="truncate">{t('by')} <AuthorName id={tp.last_post_user_id} /></div>
                        </>
                      )}
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      {composing && (
        <NewTopicWindow
          forumId={forumId}
          perms={permissions}
          onClose={() => setComposing(false)}
          onCreated={(topicId) => { setComposing(false); navigate(`/forum/topics/${topicId}`) }}
        />
      )}
    </div>
  )
}

function TypeIcon({ type, unread, locked }: { type: string; unread: boolean; locked: boolean }) {
  const cls = `w-9 h-9 rounded-lg flex items-center justify-center shrink-0 ${unread ? 'bg-primary text-white' : 'bg-surface-2 text-text-secondary'}`
  const Icon = locked ? Lock : type === 'announcement' || type === 'global' ? Megaphone : type === 'sticky' ? Pin : MessageSquare
  return <div className={cls}><Icon size={16} /></div>
}
