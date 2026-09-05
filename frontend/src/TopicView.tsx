import { useEffect, useMemo, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient, keepPreviousData } from '@tanstack/react-query'
import {
  ChevronLeft, Lock, MoreHorizontal, Send, Bell, BellOff, Shield, Check, X,
  Bookmark, CheckCircle2, Trash2, Clock, Link2, History, UserX,
  Compass, Printer, ArrowDownCircle,
} from 'lucide-react'
import { MenuDropdown, ConfirmDialog, Button, Spinner, type MenuItem } from '@ui'
import { useConfirm, useAuthStore } from '@kubuno/sdk'
import { forumApi, type Post, type BriefProfile } from './api'
import { useResolveUsers, useUser, userLabel } from './users'
import { AuthorName, AuthorAvatar } from './Author'
import { timeAgo, shortDateTime } from './helpers'
import PostBody from './PostBody'
import PostEditor from './PostEditor'
import { resolveMentionIds, type MentionRef } from './MentionPicker'
import ReactionBar from './ReactionBar'
import PollCard from './PollCard'
import { AttachPicker, PostAttachments, saveAttachments, type PendingAttachment } from './Attachments'
import MoveTopicWindow from './MoveTopicWindow'
import SplitTopicWindow from './SplitTopicWindow'
import MergeTopicWindow from './MergeTopicWindow'
import ReportPostWindow from './ReportPostWindow'
import PostRevisionsWindow from './PostRevisionsWindow'
import Pagination from './Pagination'
import Breadcrumb, { type Crumb } from './Breadcrumb'
import { useModulePrefs } from './userPrefs'

type ModWindow = 'move' | 'split' | 'merge' | null

const POSTS_PER_PAGE = 20

export default function TopicView() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const qc = useQueryClient()
  const { id: topicId = '' } = useParams()
  const me = useAuthStore(s => s.user)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  const [reply, setReply] = useState('')
  const [mentions, setMentions] = useState<MentionRef[]>([])
  const [replyAttachments, setReplyAttachments] = useState<PendingAttachment[]>([])
  const [posting, setPosting] = useState(false)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editBody, setEditBody] = useState('')
  const [headerMenu, setHeaderMenu] = useState<{ top: number; left: number } | null>(null)
  const [jumpMenu, setJumpMenu] = useState<{ top: number; left: number } | null>(null)
  const [postMenu, setPostMenu] = useState<{ post: Post; pos: { top: number; left: number } } | null>(null)
  const [watching, setWatching] = useState(false)
  const [banner, setBanner] = useState('')
  const [splitMode, setSplitMode] = useState(false)
  const [selectedPosts, setSelectedPosts] = useState<Set<string>>(new Set())
  const [modWindow, setModWindow] = useState<ModWindow>(null)
  const [reportingPost, setReportingPost] = useState<Post | null>(null)
  const [historyPost, setHistoryPost] = useState<Post | null>(null)
  const [page, setPage] = useState(1)
  const [flashedId, setFlashedId] = useState<string | null>(null)
  // Ids of ignored-author messages the reader chose to reveal anyway, for
  // this visit of the topic — reset whenever the topic itself changes.
  const [revealedIds, setRevealedIds] = useState<Set<string>>(new Set())
  const replyRef = useRef<HTMLDivElement>(null)
  const draftIdRef = useRef<string | null>(null)
  const draftTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // A different topic starts back at its first page of messages; any draft id
  // tracked for the previous topic no longer applies.
  useEffect(() => { setPage(1); draftIdRef.current = null; setMentions([]); setRevealedIds(new Set()) }, [topicId])

  // Drop tracked mentions once the composer is emptied, so a cleared reply
  // never carries stale @-picks into the next one.
  useEffect(() => { if (!reply.trim() && mentions.length) setMentions([]) }, [reply]) // eslint-disable-line react-hooks/exhaustive-deps

  // Autosave the reply draft: once it holds more than a trivial amount of
  // text, persist it in the background every 3s. Best-effort only — never
  // let a failed save interrupt typing.
  useEffect(() => {
    if (draftTimerRef.current) clearTimeout(draftTimerRef.current)
    if (!topicId || reply.trim().length <= 10) return
    draftTimerRef.current = setTimeout(() => {
      forumApi.saveDraft({ topic_id: topicId, body_md: reply.trim() })
        .then(draft => { if (draft) draftIdRef.current = draft.id })
        .catch(() => {})
    }, 3000)
    return () => { if (draftTimerRef.current) clearTimeout(draftTimerRef.current) }
  }, [reply, topicId])

  const { data: topicData, isLoading: lt } = useQuery({
    queryKey: ['forum-topic', topicId],
    queryFn: () => forumApi.getTopic(topicId),
    enabled: !!topicId,
  })
  const { data: postsData, isLoading: lp } = useQuery({
    queryKey: ['forum-posts', topicId, page],
    queryFn: () => forumApi.listPosts(topicId, { limit: POSTS_PER_PAGE, offset: (page - 1) * POSTS_PER_PAGE }),
    enabled: !!topicId,
    placeholderData: keepPreviousData,
  })

  const posts = useMemo(() => postsData?.posts ?? [], [postsData])
  const totalPosts = postsData?.total ?? 0

  const { prefs } = useModulePrefs('forum', { showSignatures: true })
  const authorIds = useMemo(() => [...new Set(posts.map(p => p.author_id))], [posts])
  const { data: briefs = [] } = useQuery({
    queryKey: ['forum-briefs', authorIds],
    queryFn: () => forumApi.getBriefProfiles(authorIds),
    enabled: authorIds.length > 0,
  })
  const briefMap = useMemo(() => new Map(briefs.map(b => [b.user_id, b])), [briefs])
  useResolveUsers(posts.map(p => p.author_id))
  // The post menu is always opened on a specific post before "Quote" can be
  // clicked, so this single hook call resolves that post's author from the
  // shared users cache for the quote attribution line.
  const quotedAuthor = useUser(postMenu?.post.author_id)

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

  // Ignore list (phpBB "foes"/zebra): a display-only fold, not a moderation
  // action — the backend keeps returning every post regardless.
  const { data: ignoredIds = [] } = useQuery({
    queryKey: ['forum-ignored'],
    queryFn: forumApi.listIgnored,
    staleTime: 5 * 60 * 1000,
  })
  const ignoredSet = useMemo(() => new Set(ignoredIds), [ignoredIds])

  const { data: allForums = [] } = useQuery({ queryKey: ['forum-all-forums'], queryFn: () => forumApi.listForums() })
  const { data: categories = [] } = useQuery({ queryKey: ['forum-categories'], queryFn: forumApi.listCategories })

  // The read marker as it stood BEFORE this visit — frozen for the whole
  // visit (staleTime: Infinity, keyed by topicId) so the "mark read" call
  // just below, which moves it forward, never overwrites the boundary this
  // page uses to compute its "first unread post" affordance.
  const { data: readAt, isFetched: readAtFetched } = useQuery({
    queryKey: ['forum-topic-readstate', topicId],
    queryFn: () => forumApi.topicReadState(topicId),
    enabled: !!topicId,
    staleTime: Infinity,
  })

  // Mark the topic read up to the latest post once posts are loaded — held
  // back until the read marker above has been captured, so that request never
  // races ahead of it (SEC/consistency: same origin, sequenced client-side).
  useEffect(() => {
    if (posts.length && topicId && readAtFetched) {
      const last = posts[posts.length - 1]
      forumApi.markRead(topicId, last.id).then(() => {
        qc.invalidateQueries({ queryKey: ['forum-readstate'] })
      }).catch(() => {})
    }
  }, [posts, topicId, qc, readAtFetched])

  // First post on the current page created after the pre-visit read marker —
  // `null` once every loaded post has already been read (or before the read
  // marker itself is known). A `readAt` of `null` means the topic was never
  // marked read before, so every loaded post counts as unread.
  const firstUnreadPost = useMemo(() => {
    if (!posts.length || readAt === undefined) return null
    if (readAt === null) return posts[0]
    const readTime = new Date(readAt).getTime()
    return posts.find(p => new Date(p.created_at).getTime() > readTime) ?? null
  }, [posts, readAt])

  const goToFirstUnread = () => {
    if (!firstUnreadPost) return
    const el = document.getElementById(`post-${firstUnreadPost.id}`)
    if (!el) return
    el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    setFlashedId(firstUnreadPost.id)
    setTimeout(() => setFlashedId(null), 2000)
  }

  // Permalinks are best-effort: the target post must be on the page that is
  // currently loaded (pagination isn't hash-aware), so a hash pointing at a
  // post outside the current page is silently ignored.
  useEffect(() => {
    const match = /^#post-(.+)$/.exec(window.location.hash)
    if (!match) return
    const id = match[1]
    if (!posts.some(p => p.id === id)) return
    const el = document.getElementById(`post-${id}`)
    if (!el) return
    el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    setFlashedId(id)
    const timer = setTimeout(() => setFlashedId(null), 2000)
    return () => clearTimeout(timer)
  }, [posts, page, topicId])

  if (lt || lp) return <div className="h-full flex items-center justify-center"><Spinner size="lg" /></div>
  if (!topicData) return null
  const { topic, permissions } = topicData
  const isMod = permissions.is_moderator || permissions.is_admin

  const forum = allForums.find(f => f.id === topic.forum_id)
  const category = forum && categories.find(c => c.id === forum.category_id)
  const crumbs: Crumb[] = [
    { label: t('forums'), to: '/forum' },
    ...(category ? [{ label: category.name, to: '/forum' }] : []),
    ...(forum ? [{ label: forum.name, to: `/forum/forums/${forum.id}` }] : []),
    { label: topic.title },
  ]
  const canReply = permissions.can_reply && (!topic.is_locked || isMod)

  // Jumpbox: every forum, grouped by category, to navigate away from this
  // topic without going back through the forum list. Purely a client-side
  // reuse of the categories/forums already loaded for the breadcrumb.
  const jumpItems: MenuItem[] = categories.flatMap(cat => {
    const catForums = allForums.filter(f => f.category_id === cat.id)
    if (!catForums.length) return []
    return [
      { type: 'label', text: cat.name } as MenuItem,
      ...catForums.map((f): MenuItem => ({
        type: 'action', label: f.name,
        onClick: () => { setJumpMenu(null); navigate(`/forum/forums/${f.id}`) },
      })),
    ]
  })

  const refresh = () => {
    qc.invalidateQueries({ queryKey: ['forum-posts', topicId] })
    qc.invalidateQueries({ queryKey: ['forum-topic', topicId] })
    qc.invalidateQueries({ queryKey: ['forum-topics', topic.forum_id] })
  }

  const submitReply = async () => {
    if (!reply.trim() || posting) return
    setPosting(true)
    try {
      const body = reply.trim()
      // Only mentions whose "@label" is still typed in the body are sent —
      // this drops picks the author deleted or overwrote by hand.
      const mention_user_ids = resolveMentionIds(body, mentions)
      const post = await forumApi.createPost(topicId, { body_md: body, ...(mention_user_ids.length ? { mention_user_ids } : {}) })
      if (replyAttachments.length) await saveAttachments(post.id, replyAttachments)
      if (draftIdRef.current) {
        forumApi.deleteDraft(draftIdRef.current).catch(() => {})
        draftIdRef.current = null
      }
      setReply('')
      setMentions([])
      setReplyAttachments([])
      refresh()
      // Jump to the last page so the freshly posted reply is on screen.
      setPage(Math.max(1, Math.ceil((totalPosts + 1) / POSTS_PER_PAGE)))
    } finally { setPosting(false) }
  }

  const saveEdit = async (id: string) => {
    if (!editBody.trim()) return
    await forumApi.updatePost(id, { body_md: editBody.trim() })
    setEditingId(null)
    refresh()
  }

  const quote = (p: Post) => {
    const authorName = quotedAuthor
      ? userLabel(quotedAuthor)
      : t('someone', { defaultValue: 'quelqu’un' })
    const attribution = t('quote_attribution', { defaultValue: '**@{{name}} a écrit :**', name: authorName })
    const quoted = p.body_md.split('\n').map(l => `> ${l}`).join('\n')
    setReply(prev => `${attribution}\n${quoted}\n\n${prev}`)
    replyRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const copyPostLink = (p: Post) => {
    const url = `${window.location.origin}/forum/topics/${topicId}#post-${p.id}`
    navigator.clipboard.writeText(url).then(() => {
      setBanner(t('link_copied', { defaultValue: 'Lien copié' }))
      setTimeout(() => setBanner(''), 3000)
    }).catch(() => {})
  }

  const removePost = async (p: Post) => {
    if (await confirm({ title: t('delete_post'), message: t('confirm_delete_post'), confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deletePost(p.id)
      refresh()
    }
  }

  const reportSent = () => {
    setReportingPost(null)
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
  const toggleIgnore = async (userId: string) => {
    if (ignoredSet.has(userId)) await forumApi.unignoreUser(userId)
    else await forumApi.ignoreUser(userId)
    qc.invalidateQueries({ queryKey: ['forum-ignored'] })
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
    const items: MenuItem[] = [
      { type: 'action', label: t('quote'), onClick: () => { setPostMenu(null); quote(p) } },
      { type: 'action', label: t('copy_link', { defaultValue: 'Copier le lien' }), icon: <Link2 size={15} />, onClick: () => { setPostMenu(null); copyPostLink(p) } },
    ]
    if (p.author_id !== me?.id) {
      items.push({
        type: 'action',
        label: ignoredSet.has(p.author_id)
          ? t('unignore_user', { defaultValue: 'Ne plus ignorer' })
          : t('ignore_user', { defaultValue: 'Ignorer cet utilisateur' }),
        icon: <UserX size={15} />,
        onClick: () => { setPostMenu(null); toggleIgnore(p.author_id) },
      })
    }
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
      if (p.edit_count > 0) {
        items.push({ type: 'action', label: t('edit_history', { defaultValue: 'Historique des modifications' }), icon: <History size={15} />, onClick: () => { setPostMenu(null); setHistoryPost(p) } })
      }
      if (!p.is_first_post) items.push({ type: 'action', label: t('delete'), danger: true, onClick: () => { setPostMenu(null); removePost(p) } })
    }
    if (isMod && !p.is_first_post) {
      items.push({ type: 'action', label: t('mod_remove'), icon: <Trash2 size={15} />, danger: true, onClick: () => { setPostMenu(null); modRemovePost(p) } })
    }
    items.push({ type: 'separator' })
    items.push({ type: 'action', label: t('report'), onClick: () => { setPostMenu(null); setReportingPost(p) } })
    return items
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <Breadcrumb items={crumbs} />
        {/* Header */}
        <div className="flex items-center gap-2 mb-4">
          <button onClick={() => navigate(`/forum/forums/${topic.forum_id}`)} className="p-1.5 rounded hover:bg-surface-1 text-text-secondary no-print" title={t('back')}>
            <ChevronLeft size={18} />
          </button>
          {jumpItems.length > 0 && (
            <button onClick={(e) => setJumpMenu({ top: e.clientY, left: e.clientX })} title={t('jump_to_forum', { defaultValue: 'Accéder à un forum' })} className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary no-print">
              <Compass size={16} />
            </button>
          )}
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
          <button onClick={() => window.print()} title={t('print', { defaultValue: 'Imprimer' })} className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary no-print">
            <Printer size={16} />
          </button>
          {isMod && (
            <button onClick={(e) => setHeaderMenu({ top: e.clientY, left: e.clientX })} title={t('moderation')} className="p-2 rounded-lg hover:bg-surface-1 text-text-secondary no-print">
              <Shield size={16} />
            </button>
          )}
        </div>

        {banner && <div className="mb-3 px-3 py-2 rounded-lg bg-success-light text-success text-sm">{banner}</div>}

        {firstUnreadPost && (
          <button onClick={goToFirstUnread}
            className="mb-3 inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-primary-light text-primary hover:brightness-95 no-print">
            <ArrowDownCircle size={13} /> {t('jump_to_unread', { defaultValue: 'Aller au premier message non lu' })}
          </button>
        )}

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
          {posts.map((p) => (
            <article key={p.id} id={`post-${p.id}`} className={`rounded-xl border bg-surface-0 overflow-hidden print-avoid transition-colors duration-700 ${flashedId === p.id ? 'border-primary bg-primary-light ring-2 ring-primary' : topic.solution_post_id === p.id ? 'border-success' : selectedPosts.has(p.id) ? 'border-primary' : 'border-border'}`}>
              {ignoredSet.has(p.author_id) && !revealedIds.has(p.id) ? (
                <div className="flex items-center gap-2 px-3 py-2.5 text-sm text-text-tertiary">
                  <UserX size={14} className="shrink-0" />
                  <span className="flex-1">{t('ignored_post_hidden', { defaultValue: "Message masqué d'un utilisateur ignoré" })}</span>
                  <button
                    onClick={() => setRevealedIds(prev => { const n = new Set(prev); n.add(p.id); return n })}
                    className="text-primary hover:underline text-xs font-medium shrink-0"
                  >
                    {t('show_hidden_post', { defaultValue: 'Afficher' })}
                  </button>
                </div>
              ) : (
              <div className="flex">
                {/* Author column */}
                <div className="w-36 shrink-0 bg-surface-1 p-3 border-r border-border hidden sm:flex flex-col items-center text-center gap-1">
                  <button onClick={() => navigate(`/forum/profiles/${p.author_id}`)} className="flex flex-col items-center gap-1 group w-full min-w-0">
                    <AuthorAvatar id={p.author_id} size={48} />
                    <div className="text-sm font-medium text-text-primary truncate w-full group-hover:text-primary"><AuthorName id={p.author_id} /></div>
                  </button>
                  <AuthorMeta brief={briefMap.get(p.author_id)} />
                  {p.is_first_post && <span className="text-[10px] uppercase tracking-wide text-text-tertiary">{t('topic')}</span>}
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
                      {/* Only its author and the moderators are served a message
                          that is still waiting; saying so is what stops the
                          author from posting it a second time. */}
                      {!p.is_approved && (
                        <div className="flex items-center gap-1.5 mb-2 text-xs font-medium text-warning">
                          <Clock size={14} /> {t('post_pending')}
                        </div>
                      )}
                      <PostBody body={p.body_md} />
                      <PostAttachments postId={p.id} />
                      <ReactionBar postId={p.id} initial={reactions[p.id] ?? []} />
                      {prefs.showSignatures && briefMap.get(p.author_id)?.signature_md && (
                        <div className="mt-3 pt-2 border-t border-border/50">
                          <PostBody body={briefMap.get(p.author_id)!.signature_md!} signature />
                        </div>
                      )}
                    </>
                  )}
                </div>
              </div>
              )}
            </article>
          ))}
        </div>

        <div className="no-print">
          <Pagination page={page} pageSize={POSTS_PER_PAGE} total={totalPosts} onPage={setPage} />
        </div>

        {/* Split mode action bar */}
        {splitMode && (
          <div className="sticky bottom-2 mt-3 flex items-center gap-2 bg-surface-0 border border-primary rounded-xl px-3 py-2 shadow-lg no-print">
            <span className="text-sm text-text-secondary flex-1">{t('post_count', { count: selectedPosts.size })}</span>
            <Button variant="ghost" size="sm" onClick={() => { setSplitMode(false); setSelectedPosts(new Set()) }}>{t('cancel')}</Button>
            <Button variant="primary" size="sm" disabled={selectedPosts.size === 0} onClick={() => setModWindow('split')}>{t('split_topic')}</Button>
          </div>
        )}

        {/* Reply composer */}
        {canReply && !splitMode && (
          <div ref={replyRef} className="mt-5 no-print">
            <PostEditor value={reply} onChange={setReply} placeholder={t('write_reply')} rows={5} onSubmit={submitReply}
              onMention={(m) => setMentions(prev => [...prev, m])} />
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
      {jumpMenu && <MenuDropdown items={jumpItems} pos={jumpMenu} onClose={() => setJumpMenu(null)} />}

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
      {reportingPost && (
        <ReportPostWindow postId={reportingPost.id} onClose={() => setReportingPost(null)} onSent={reportSent} />
      )}
      {historyPost && (
        <PostRevisionsWindow post={historyPost} onClose={() => setHistoryPost(null)} />
      )}

      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}

/** Rank badge/title and post count under an author's name in the post column. */
function AuthorMeta({ brief }: { brief?: BriefProfile }) {
  const { t } = useTranslation('forum')
  if (!brief) return null
  const title = brief.custom_title || brief.rank_title
  return (
    <>
      {(brief.rank_badge || title) && (
        <div className="text-[10px] text-text-tertiary flex items-center gap-1 leading-tight max-w-full">
          {brief.rank_badge && <span>{brief.rank_badge}</span>}
          {title && <span className="truncate">{title}</span>}
        </div>
      )}
      <div className="text-[10px] text-text-tertiary">{t('member_posts', { count: brief.post_count })}</div>
    </>
  )
}
