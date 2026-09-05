import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient, keepPreviousData } from '@tanstack/react-query'
import {
  MessagesSquare, Lock, Pin, Megaphone, MessageSquare, Eye, ChevronLeft, Plus, Bell, BellOff, CheckCheck, ListChecks,
} from 'lucide-react'
import { Spinner, Button, Badge, ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi, type Topic } from './api'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo } from './helpers'
import NewTopicWindow from './NewTopicWindow'
import Pagination from './Pagination'
import Breadcrumb, { type Crumb } from './Breadcrumb'

const TOPICS_PER_PAGE = 30

export default function ForumView() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { id: forumId = '' } = useParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const [composing, setComposing] = useState(false)
  const [watching, setWatching] = useState(false)
  const [page, setPage] = useState(1)
  const [selectMode, setSelectMode] = useState(false)
  const [selectedTopics, setSelectedTopics] = useState<Set<string>>(new Set())
  const [bulkBusy, setBulkBusy] = useState(false)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  // A different forum starts back at its first page, and drops any in-progress
  // multi-select — the selected ids would no longer even belong to this list.
  useEffect(() => { setPage(1); setSelectMode(false); setSelectedTopics(new Set()) }, [forumId])

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
    queryKey: ['forum-topics', forumId, page],
    queryFn: () => forumApi.listTopics(forumId, { limit: TOPICS_PER_PAGE, offset: (page - 1) * TOPICS_PER_PAGE }),
    enabled: !!forumId,
    placeholderData: keepPreviousData,
  })
  const { data: allForums = [] } = useQuery({ queryKey: ['forum-all-forums'], queryFn: () => forumApi.listForums() })
  const { data: categories = [] } = useQuery({ queryKey: ['forum-categories'], queryFn: forumApi.listCategories })
  const subForums = useMemo(() => allForums.filter(f => f.parent_forum_id === forumId), [allForums, forumId])
  const { data: readState = [] } = useQuery({
    queryKey: ['forum-readstate', forumId],
    queryFn: () => forumApi.forumReadState(forumId),
    enabled: !!forumId,
  })

  const topics = topicsData?.topics ?? []
  const totalTopics = topicsData?.total ?? 0
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

  const markRead = async () => {
    await forumApi.markForumRead(forumId)
    queryClient.invalidateQueries({ queryKey: ['forum-readstate', forumId] })
    queryClient.invalidateQueries({ queryKey: ['forum-topics', forumId] })
  }

  const toggleTopicSelect = (id: string) => {
    setSelectedTopics(prev => {
      const next = new Set(prev)
      next.has(id) ? next.delete(id) : next.add(id)
      return next
    })
  }

  const exitSelectMode = () => { setSelectMode(false); setSelectedTopics(new Set()) }

  const runBulkAction = async (action: 'lock' | 'unlock' | 'delete') => {
    if (selectedTopics.size === 0 || bulkBusy) return
    if (action === 'delete') {
      const ok = await confirm({
        title: t('delete_topic'),
        message: t('confirm_bulk_delete_topics', { defaultValue: 'Supprimer {{count}} sujet(s) sélectionné(s) ?', count: selectedTopics.size }),
        confirmLabel: t('delete'),
        variant: 'danger',
      })
      if (!ok) return
    }
    setBulkBusy(true)
    try {
      await forumApi.bulkModerateTopics({ topic_ids: [...selectedTopics], action })
      queryClient.invalidateQueries({ queryKey: ['forum-topics', forumId] })
      exitSelectMode()
    } finally {
      setBulkBusy(false)
    }
  }

  if (lf || lt) return <div className="h-full flex items-center justify-center"><Spinner size="lg" /></div>
  if (!forumData) return null
  const { forum, permissions } = forumData
  const isMod = permissions.is_moderator || permissions.is_admin

  const category = categories.find(c => c.id === forum.category_id)
  const parentForum = forum.parent_forum_id ? allForums.find(f => f.id === forum.parent_forum_id) : undefined
  const crumbs: Crumb[] = [
    { label: t('forums'), to: '/forum' },
    ...(category ? [{ label: category.name, to: '/forum' }] : []),
    ...(parentForum ? [{ label: parentForum.name, to: `/forum/forums/${parentForum.id}` }] : []),
    { label: forum.name },
  ]

  const isUnread = (tp: Topic) => {
    if (!tp.last_post_id) return false
    if (!readMap.has(tp.id)) return true
    return readMap.get(tp.id) !== tp.last_post_id
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-4xl mx-auto px-4 py-5">
        <Breadcrumb items={crumbs} />
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
          <button onClick={markRead} title={t('mark_read', { defaultValue: 'Mark read' })}
            className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary">
            <CheckCheck size={16} />
          </button>
          <button onClick={toggleWatch} title={watching ? t('unsubscribe') : t('subscribe')}
            className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary">
            {watching ? <BellOff size={16} /> : <Bell size={16} />}
          </button>
          {isMod && (
            <Button
              variant={selectMode ? 'secondary' : 'ghost'}
              size="sm"
              icon={<ListChecks size={15} />}
              onClick={() => (selectMode ? exitSelectMode() : setSelectMode(true))}
            >
              {selectMode ? t('cancel') : t('select_topics', { defaultValue: 'Sélectionner' })}
            </Button>
          )}
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
              {topics.map(tp => {
                const rowContent = (
                  <>
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
                  </>
                )
                return (
                  <li key={tp.id}>
                    {selectMode ? (
                      <div
                        role="button"
                        tabIndex={0}
                        onClick={() => toggleTopicSelect(tp.id)}
                        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleTopicSelect(tp.id) } }}
                        className="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-surface-1 cursor-pointer"
                      >
                        <input
                          type="checkbox"
                          checked={selectedTopics.has(tp.id)}
                          onClick={(e) => e.stopPropagation()}
                          onChange={() => toggleTopicSelect(tp.id)}
                          className="shrink-0"
                        />
                        {rowContent}
                      </div>
                    ) : (
                      <button onClick={() => navigate(`/forum/topics/${tp.id}`)} className="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-surface-1">
                        {rowContent}
                      </button>
                    )}
                  </li>
                )
              })}
            </ul>
          )}
        </div>

        <Pagination page={page} pageSize={TOPICS_PER_PAGE} total={totalTopics} onPage={setPage} />

        {/* Bulk moderation action bar */}
        {selectMode && (
          <div className="sticky bottom-2 mt-3 flex items-center gap-2 bg-surface-0 border border-primary rounded-xl px-3 py-2 shadow-lg">
            <span className="text-sm text-text-secondary flex-1">
              {t('topics_selected', { defaultValue: '{{count}} sujet(s) sélectionné(s)', count: selectedTopics.size })}
            </span>
            <Button variant="ghost" size="sm" onClick={exitSelectMode}>{t('cancel')}</Button>
            <Button variant="secondary" size="sm" disabled={selectedTopics.size === 0 || bulkBusy} loading={bulkBusy} onClick={() => runBulkAction('unlock')}>
              {t('unlock_topic')}
            </Button>
            <Button variant="secondary" size="sm" disabled={selectedTopics.size === 0 || bulkBusy} loading={bulkBusy} onClick={() => runBulkAction('lock')}>
              {t('lock_topic')}
            </Button>
            <Button variant="danger" size="sm" disabled={selectedTopics.size === 0 || bulkBusy} loading={bulkBusy} onClick={() => runBulkAction('delete')}>
              {t('delete')}
            </Button>
          </div>
        )}
      </div>

      {composing && (
        <NewTopicWindow
          forumId={forumId}
          perms={permissions}
          onClose={() => setComposing(false)}
          onCreated={(topicId) => { setComposing(false); navigate(`/forum/topics/${topicId}`) }}
        />
      )}

      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}

function TypeIcon({ type, unread, locked }: { type: string; unread: boolean; locked: boolean }) {
  const cls = `w-9 h-9 rounded-lg flex items-center justify-center shrink-0 ${unread ? 'bg-primary text-white' : 'bg-surface-2 text-text-secondary'}`
  const Icon = locked ? Lock : type === 'announcement' || type === 'global' ? Megaphone : type === 'sticky' ? Pin : MessageSquare
  return <div className={cls}><Icon size={16} /></div>
}
