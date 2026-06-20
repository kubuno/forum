import { useEffect, useMemo, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  ChevronLeft, Lock, MoreHorizontal, Send, Bell, BellOff, Shield, Check, X,
  Bookmark, CheckCircle2, Trash2,
} from 'lucide-react'
import { MenuDropdown, ConfirmDialog, Button, Spinner, type MenuItem } from '@ui'
import { useConfirm, prompt, useAuthStore } from '@kubuno/sdk'
import { forumApi, type Post } from './api'
import { useResolveUsers } from './users'
import { AuthorName, AuthorAvatar } from './Author'
import { timeAgo, shortDateTime } from './helpers'
import PostBody from './PostBody'
import PostEditor from './PostEditor'
import ReactionBar from './ReactionBar'
import PollCard from './PollCard'
import { AttachPicker, PostAttachments, saveAttachments, type PendingAttachment } from './Attachments'
import MoveTopicWindow from './MoveTopicWindow'
import SplitTopicWindow from './SplitTopicWindow'
import MergeTopicWindow from './MergeTopicWindow'

type ModWindow = 'move' | 'split' | 'merge' | null

export default function TopicView() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const qc = useQueryClient()
  const { id: topicId = '' } = useParams()
  const me = useAuthStore(s => s.user)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  const [reply, setReply] = useState('')
  const [replyAttachments, setReplyAttachments] = useState<PendingAttachment[]>([])
  const [posting, setPosting] = useState(false)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editBody, setEditBody] = useState('')
  const [headerMenu, setHeaderMenu] = useState<{ top: number; left: number } | null>(null)
  const [postMenu, setPostMenu] = useState<{ post: Post; pos: { top: number; left: number } } | null>(null)
  const [watching, setWatching] = useState(false)
  const [banner, setBanner] = useState('')
  const [splitMode, setSplitMode] = useState(false)
  const [selectedPosts, setSelectedPosts] = useState<Set<string>>(new Set())
  const [modWindow, setModWindow] = useState<ModWindow>(null)
  const replyRef = useRef<HTMLDivElement>(null)

  const { data: topicData, isLoading: lt } = useQuery({
    queryKey: ['forum-topic', topicId],
    queryFn: () => forumApi.getTopic(topicId),
    enabled: !!topicId,
  })
  const { data: postsData, isLoading: lp } = useQuery({
    queryKey: ['forum-posts', topicId],
    queryFn: () => forumApi.listPosts(topicId, { limit: 100 }),
    enabled: !!topicId,
  })

  const posts = useMemo(() => postsData?.posts ?? [], [postsData])
  useResolveUsers(posts.map(p => p.author_id))

  const { data: reactions = {} } = useQuery({
    queryKey: ['forum-reactions', topicId],
    queryFn: () => forumApi.topicReactions(topicId),
    enabled: !!topicId,
  })
  const { data: topicTags = [] } = useQuery({
    queryKey: ['forum-topictags', topicId],
    queryFn: () => forumApi.topicTags(topicId),
    enabled: !!topicId,
  })
  const { data: bookmarks = [] } = useQuery({ queryKey: ['forum-bookmarks'], queryFn: forumApi.listBookmarks })
  const isBookmarked = bookmarks.some(b => b.id === topicId)

  // Mark the topic read up to the latest post once posts are loaded.
  useEffect(() => {
    if (posts.length && topicId) {
      const last = posts[posts.length - 1]
      forumApi.markRead(topicId, last.id).then(() => {
        qc.invalidateQueries({ queryKey: ['forum-readstate'] })
      }).catch(() => {})
    }
  }, [posts, topicId, qc])

  if (lt || lp) return <div className="h-full flex items-center justify-center"><Spinner size="lg" /></div>
  if (!topicData) return null
  const { topic, permissions } = topicData
  const isMod = permissions.is_moderator || permissions.is_admin
  const canReply = permissions.can_reply && (!topic.is_locked || isMod)

  const refresh = () => {
    qc.invalidateQueries({ queryKey: ['forum-posts', topicId] })
    qc.invalidateQueries({ queryKey: ['forum-topic', topicId] })
    qc.invalidateQueries({ queryKey: ['forum-topics', topic.forum_id] })
  }

  const submitReply = async () => {
    if (!reply.trim() || posting) return
    setPosting(true)
    try {
      const post = await forumApi.createPost(topicId, { body_md: reply.trim() })
      if (replyAttachments.length) await saveAttachments(post.id, replyAttachments)
      setReply('')
      setReplyAttachments([])
      refresh()
    } finally { setPosting(false) }
  }

  const saveEdit = async (id: string) => {
    if (!editBody.trim()) return
    await forumApi.updatePost(id, { body_md: editBody.trim() })
    setEditingId(null)
    refresh()
  }

  const quote = (p: Post) => {
    const quoted = p.body_md.split('\n').map(l => `> ${l}`).join('\n')
    setReply(prev => `${quoted}\n\n${prev}`)
    replyRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const removePost = async (p: Post) => {
    if (await confirm({ title: t('delete_post'), message: t('confirm_delete_post'), confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deletePost(p.id)
      refresh()
    }
  }

  const reportPost = async (p: Post) => {
    const reason = await prompt({ title: t('report_post'), placeholder: t('report_reason'), confirmLabel: t('report') })
    if (!reason?.trim()) return
    await forumApi.reportPost(p.id, reason.trim())
    setBanner(t('report_sent'))
    setTimeout(() => setBanner(''), 3000)
  }

  const toggleWatch = async () => {
    if (watching) { await forumApi.unsubscribeTopic(topicId); setWatching(false) }
    else { await forumApi.subscribeTopic(topicId); setWatching(true) }
  }

  const toggleBookmark = async () => {
    await forumApi.toggleBookmark(topicId)
    qc.invalidateQueries({ queryKey: ['forum-bookmarks'] })
  }
  const markSolution = async (p: Post) => {
    if (topic.solution_post_id === p.id) await forumApi.clearSolution(topicId)
    else await forumApi.setSolution(topicId, p.id)
    refresh()
  }
  const modRemovePost = async (p: Post) => {
    await forumApi.removePost(p.id); refresh()
  }

  const toggleLock = async () => { await forumApi.lockTopic(topicId, !topic.is_locked); refresh() }
  const togglePin = async () => {
    await forumApi.updateTopic(topicId, { topic_type: topic.topic_type === 'normal' ? 'sticky' : 'normal' })
    refresh()
  }
  const removeTopic = async () => {
    if (await confirm({ title: t('delete_topic'), message: t('confirm_delete_topic'), confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteTopic(topicId)
      navigate(`/forum/forums/${topic.forum_id}`)
    }
  }

  const headerItems: MenuItem[] = [
    { type: 'action', label: topic.is_locked ? t('unlock_topic') : t('lock_topic'), icon: <Lock size={15} />, onClick: () => { setHeaderMenu(null); toggleLock() } },
    { type: 'action', label: topic.topic_type === 'normal' ? t('pin_topic') : t('unpin_topic'), onClick: () => { setHeaderMenu(null); togglePin() } },
    { type: 'separator' },
    { type: 'action', label: t('move_topic'), onClick: () => { setHeaderMenu(null); setModWindow('move') } },
    { type: 'action', label: t('split_topic'), onClick: () => { setHeaderMenu(null); setSplitMode(true); setSelectedPosts(new Set()) } },
    { type: 'action', label: t('merge_topic'), onClick: () => { setHeaderMenu(null); setModWindow('merge') } },
    { type: 'separator' },
    { type: 'action', label: t('delete_topic'), danger: true, onClick: () => { setHeaderMenu(null); removeTopic() } },
  ]

  const canMarkSolution = topic.author_id === me?.id || isMod
  const postItems = (p: Post): MenuItem[] => {
    const items: MenuItem[] = [{ type: 'action', label: t('quote'), onClick: () => { setPostMenu(null); quote(p) } }]
    if (canMarkSolution && !p.is_first_post) {
      items.push({
        type: 'action',
        label: topic.solution_post_id === p.id ? t('unmark_solution') : t('mark_solution'),
        icon: <CheckCircle2 size={15} />,
        onClick: () => { setPostMenu(null); markSolution(p) },
      })
    }
    if (p.author_id === me?.id || isMod) {
      items.push({ type: 'action', label: t('edit'), onClick: () => { setPostMenu(null); setEditingId(p.id); setEditBody(p.body_md) } })
      if (!p.is_first_post) items.push({ type: 'action', label: t('delete'), danger: true, onClick: () => { setPostMenu(null); removePost(p) } })
    }
    if (isMod && !p.is_first_post) {
      items.push({ type: 'action', label: t('mod_remove'), icon: <Trash2 size={15} />, danger: true, onClick: () => { setPostMenu(null); modRemovePost(p) } })
    }
    items.push({ type: 'separator' })
    items.push({ type: 'action', label: t('report'), onClick: () => { setPostMenu(null); reportPost(p) } })
    return items
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        {/* Header */}
        <div className="flex items-center gap-2 mb-4">
          <button onClick={() => navigate(`/forum/forums/${topic.forum_id}`)} className="p-1.5 rounded hover:bg-surface-1 text-text-secondary no-print" title={t('back')}>
            <ChevronLeft size={18} />
          </button>
          <h1 className="text-lg font-semibold text-text-primary flex-1 min-w-0 truncate flex items-center gap-2">
            {topic.is_locked && <Lock size={15} className="text-text-tertiary shrink-0" />}
            {topic.is_solved && (
              <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[11px] font-medium bg-success-light text-success shrink-0">
                <CheckCircle2 size={12} /> {t('solved')}
              </span>
            )}
            {topic.prefix && <span className="px-1.5 py-0.5 rounded text-[11px] font-medium bg-primary-light text-primary shrink-0">{topic.prefix}</span>}
            <span className="truncate">{topic.title}</span>
          </h1>
          <button onClick={toggleBookmark} title={t('bookmark')} className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary no-print">
            <Bookmark size={16} className={isBookmarked ? 'fill-primary text-primary' : ''} />
          </button>
          <button onClick={toggleWatch} title={watching ? t('unsubscribe') : t('subscribe')} className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary no-print">
            {watching ? <BellOff size={16} /> : <Bell size={16} />}
          </button>
          {isMod && (
            <button onClick={(e) => setHeaderMenu({ top: e.clientY, left: e.clientX })} title={t('moderation')} className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary no-print">
              <Shield size={16} />
            </button>
          )}
        </div>

        {banner && <div className="mb-3 px-3 py-2 rounded-lg bg-success-light text-success text-sm">{banner}</div>}

        {topicTags.length > 0 && (
          <div className="flex items-center gap-1.5 flex-wrap mb-3">
            {topicTags.map(tag => (
              <span key={tag.id} className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
                style={{ backgroundColor: tag.color + '22', color: tag.color }}>
                <span className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: tag.color }} />{tag.name}
              </span>
            ))}
          </div>
        )}

        <PollCard topicId={topicId} />

        {/* Posts */}
        <div className="space-y-3">
          {posts.map((p, i) => (
            <article key={p.id} className={`rounded-xl border bg-surface-0 overflow-hidden print-avoid ${topic.solution_post_id === p.id ? 'border-success' : selectedPosts.has(p.id) ? 'border-primary' : 'border-border'}`}>
              <div className="flex">
                {/* Author column */}
                <div className="w-36 shrink-0 bg-surface-1 p-3 border-r border-border hidden sm:flex flex-col items-center text-center gap-1">
                  <AuthorAvatar id={p.author_id} size={48} />
                  <div className="text-sm font-medium text-text-primary truncate w-full"><AuthorName id={p.author_id} /></div>
                  {i === 0 && <span className="text-[10px] uppercase tracking-wide text-text-tertiary">{t('topic')}</span>}
                </div>
                {/* Body column */}
                <div className="flex-1 min-w-0 p-3">
                  <div className="flex items-center gap-2 text-xs text-text-tertiary mb-2">
                    {splitMode && !p.is_first_post && (
                      <input type="checkbox" checked={selectedPosts.has(p.id)} onChange={(e) => {
                        setSelectedPosts(prev => { const n = new Set(prev); e.target.checked ? n.add(p.id) : n.delete(p.id); return n })
                      }} />
                    )}
                    <span className="sm:hidden font-medium text-text-secondary"><AuthorName id={p.author_id} /></span>
                    <span title={shortDateTime(p.created_at)}>{timeAgo(p.created_at)}</span>
                    {p.edit_count > 0 && <span className="italic">· {t('edited')}</span>}
                    <button onClick={(e) => setPostMenu({ post: p, pos: { top: e.clientY, left: e.clientX } })}
                      className="ml-auto p-1 rounded hover:bg-surface-2 text-text-tertiary hover:text-text-primary no-print" title={t('post')}>
                      <MoreHorizontal size={16} />
                    </button>
                  </div>
                  {editingId === p.id ? (
                    <div className="space-y-2">
                      <PostEditor value={editBody} onChange={setEditBody} rows={6} />
                      <div className="flex justify-end gap-2">
                        <Button variant="ghost" size="sm" icon={<X size={14} />} onClick={() => setEditingId(null)}>{t('cancel')}</Button>
                        <Button variant="primary" size="sm" icon={<Check size={14} />} onClick={() => saveEdit(p.id)}>{t('save')}</Button>
                      </div>
                    </div>
                  ) : (
                    <>
                      {topic.solution_post_id === p.id && (
                        <div className="flex items-center gap-1.5 mb-2 text-xs font-medium text-success">
                          <CheckCircle2 size={14} /> {t('solution')}
                        </div>
                      )}
                      <PostBody body={p.body_md} />
                      <PostAttachments postId={p.id} />
                      <ReactionBar postId={p.id} initial={reactions[p.id] ?? []} />
                    </>
                  )}
                </div>
              </div>
            </article>
          ))}
        </div>

        {/* Split mode action bar */}
        {splitMode && (
          <div className="sticky bottom-2 mt-3 flex items-center gap-2 bg-surface-0 border border-primary rounded-xl px-3 py-2 shadow-lg">
            <span className="text-sm text-text-secondary flex-1">{t('post_count', { count: selectedPosts.size })}</span>
            <Button variant="ghost" size="sm" onClick={() => { setSplitMode(false); setSelectedPosts(new Set()) }}>{t('cancel')}</Button>
            <Button variant="primary" size="sm" disabled={selectedPosts.size === 0} onClick={() => setModWindow('split')}>{t('split_topic')}</Button>
          </div>
        )}

        {/* Reply composer */}
        {canReply && !splitMode && (
          <div ref={replyRef} className="mt-5 no-print">
            <PostEditor value={reply} onChange={setReply} placeholder={t('write_reply')} rows={5} onSubmit={submitReply} />
            <div className="flex items-center justify-between gap-2 mt-2">
              {permissions.can_attach ? <AttachPicker value={replyAttachments} onChange={setReplyAttachments} /> : <span />}
              <Button variant="primary" icon={<Send size={15} />} loading={posting} disabled={!reply.trim()} onClick={submitReply}>{t('reply')}</Button>
            </div>
          </div>
        )}
        {topic.is_locked && !isMod && (
          <div className="mt-5 flex items-center justify-center gap-2 text-sm text-text-tertiary py-4">
            <Lock size={15} /> {t('locked')}
          </div>
        )}
      </div>

      {headerMenu && <MenuDropdown items={headerItems} pos={headerMenu} onClose={() => setHeaderMenu(null)} />}
      {postMenu && <MenuDropdown items={postItems(postMenu.post)} pos={postMenu.pos} onClose={() => setPostMenu(null)} />}

      {modWindow === 'move' && (
        <MoveTopicWindow topicId={topicId} currentForumId={topic.forum_id} onClose={() => setModWindow(null)} onMoved={() => { setModWindow(null); refresh() }} />
      )}
      {modWindow === 'split' && (
        <SplitTopicWindow topicId={topicId} currentForumId={topic.forum_id} postIds={[...selectedPosts]}
          onClose={() => setModWindow(null)}
          onSplit={(newId) => { setModWindow(null); setSplitMode(false); setSelectedPosts(new Set()); navigate(`/forum/topics/${newId}`) }} />
      )}
      {modWindow === 'merge' && (
        <MergeTopicWindow topicId={topicId} forumId={topic.forum_id} onClose={() => setModWindow(null)} onMerged={() => { setModWindow(null); refresh() }} />
      )}

      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
