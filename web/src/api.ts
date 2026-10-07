import { api as apiClient } from '@kubuno/sdk'

// ── Types ─────────────────────────────────────────────────────────────────────

export interface Category {
  id: string
  name: string
  description: string | null
  position: number
  created_at: string
  updated_at: string
}

export interface Forum {
  id: string
  category_id: string
  parent_forum_id: string | null
  name: string
  description: string | null
  position: number
  is_locked: boolean
  topic_count: number
  post_count: number
  last_post_id: string | null
  last_post_at: string | null
  last_post_user_id: string | null
  color: string | null
  icon: string | null
  is_readonly: boolean
  rules_md: string | null
  created_at: string
  updated_at: string
}

export type TopicType = 'normal' | 'sticky' | 'announcement' | 'global'

export interface Topic {
  id: string
  forum_id: string
  author_id: string
  title: string
  slug: string
  topic_type: TopicType
  is_locked: boolean
  is_approved: boolean
  approved_at: string | null
  approved_by: string | null
  view_count: number
  reply_count: number
  first_post_id: string | null
  last_post_id: string | null
  last_post_at: string | null
  last_post_user_id: string | null
  is_solved: boolean
  solution_post_id: string | null
  is_question: boolean
  prefix: string | null
  is_deleted: boolean
  deleted_at: string | null
  deleted_by: string | null
  created_at: string
  updated_at: string
}

export interface Post {
  id: string
  topic_id: string
  forum_id: string
  author_id: string
  body_md: string
  reply_to_post_id: string | null
  is_first_post: boolean
  is_approved: boolean
  approved_at: string | null
  approved_by: string | null
  edited_at: string | null
  edited_by: string | null
  edit_reason: string | null
  edit_count: number
  is_deleted: boolean
  like_count: number
  created_at: string
  updated_at: string
}

/** A past version of a post's body, archived just before an edit overwrote
 *  it. Only the post's author and moderators can fetch these. */
export interface PostRevision {
  id: string
  post_id: string
  body_md: string
  edited_by: string | null
  edit_reason: string | null
  created_at: string
}

export interface Attachment {
  id: string
  post_id: string
  file_id: string | null
  filename: string
  mime_type: string | null
  size_bytes: number | null
  created_at: string
}

export interface ForumPerms {
  can_post?: boolean
  can_reply: boolean
  can_attach: boolean
  is_moderator: boolean
  is_admin: boolean
}

export interface Report {
  id: string
  post_id: string
  reporter_id: string
  reason: string
  reason_id: string | null
  status: 'open' | 'resolved' | 'rejected'
  handled_by: string | null
  handled_at: string | null
  created_at: string
}

/** A predefined reason an admin curates for the report chip picker. */
export interface ReportReason {
  id: string
  title: string
  description: string | null
  position: number
  created_at: string
}

/** An admin-curated word substituted server-side in every post body at
 *  render time (word censor). */
export interface CensoredWord {
  id: string
  pattern: string
  replacement: string
  created_at: string
}

export type ProfileFieldType = 'text' | 'textarea' | 'bool' | 'url' | 'date' | 'dropdown'
export type ProfileFieldVisibility = 'public' | 'registered'

/** An admin-curated custom profile field definition. Answers
 *  are always rendered as plain, escaped text — never Markdown/HTML. */
export interface ProfileField {
  id: string
  key: string
  label: string
  field_type: ProfileFieldType
  /** Only meaningful for `field_type === 'dropdown'`. */
  options: string[] | null
  position: number
  visibility: ProfileFieldVisibility
  show_on_posts: boolean
  required: boolean
  created_at: string
}

/** One member's answer to one field — always plain text on the wire. */
export interface ProfileFieldValue {
  user_id: string
  field_id: string
  value: string
}

/** One question/answer pair of the editable FAQ, admin-curated
 *  and read by every member. `answer_md` is Markdown, rendered client-side
 *  with `PostBody` — the same sanitized renderer as post bodies. */
export interface FaqEntry {
  id: string
  question: string
  answer_md: string
  position: number
  created_at: string
  updated_at: string
}

/** One entry of the approval queue: a contribution held back by the instance's
 *  moderation policy, with enough context to decide without opening it. */
export interface PendingPost {
  id: string
  topic_id: string
  forum_id: string
  author_id: string
  body_md: string
  /** True when releasing this message also releases a whole new topic. */
  is_first_post: boolean
  created_at: string
  topic_title: string
  forum_name: string
}

export interface Moderator {
  forum_id: string
  user_id: string
  created_at: string
}

export interface TeamMember {
  forum_id: string
  forum_name: string
  user_id: string
}

export interface Rank {
  id: string
  title: string
  min_posts: number
  is_special: boolean
  badge: string | null
  created_at: string
}

export interface UserProfile {
  user_id: string
  post_count: number
  rank_id: string | null
  signature_md: string | null
  bio_md: string | null
  location: string | null
  website: string | null
  custom_title: string | null
  likes_received: number
  likes_given: number
  topic_count: number
  last_seen_at: string | null
  created_at: string
  updated_at: string
}

export interface BriefProfile {
  user_id: string
  post_count: number
  custom_title: string | null
  rank_title: string | null
  rank_badge: string | null
  signature_md: string | null
}

export interface Member {
  user_id: string
  post_count: number
  rank_title: string | null
  rank_badge: string | null
  last_seen_at: string | null
  created_at: string
}

/** A user currently online, with a best-effort readable location (their
 * current topic's title) — `null` when unknown or when the caller isn't
 * allowed to see that topic's forum. */
export interface OnlineUser {
  user_id: string
  path: string | null
}

export interface LeaderboardEntry {
  user_id: string
  post_count: number
}

export interface Permission {
  id: string
  forum_id: string
  role: 'guest' | 'user' | 'moderator'
  can_view: boolean
  can_post: boolean
  can_reply: boolean
  can_attach: boolean
}

/** A group-scoped permission grant for a forum. Grants are additive: they can
 *  only open a restricted forum to a group, never revoke role-based access. */
export interface GroupPermission {
  id: string
  forum_id: string
  group_id: string
  can_view: boolean
  can_post: boolean
  can_reply: boolean
  can_attach: boolean
}

/** A directory group of the instance (admin-managed, shared across modules). */
export interface DirectoryGroup {
  id: string
  name: string
}

export interface ReadState {
  topic_id: string
  last_read_post_id: string | null
}

export interface Subscription {
  id: string
  user_id: string
  topic_id: string | null
  forum_id: string | null
  created_at: string
}

export interface SearchHit {
  post_id: string
  topic_id: string
  forum_id: string
  author_id: string
  topic_title: string
  topic_slug: string
  snippet: string
  created_at: string
}

export interface UserBrief {
  id: string
  username: string
  display_name: string
  avatar_url: string | null
}

export interface EmojiAgg { emoji: string; count: number; me: boolean }

/** Users who reacted to a post with a given emoji ("who reacted" tooltip). */
export interface ReactionUsers { emoji: string; users: string[] }

export interface ForumNotification {
  id: string
  kind: string
  actor_id: string | null
  topic_id: string | null
  post_id: string | null
  extra: string | null
  is_read: boolean
  responder_count: number
  created_at: string
}

export interface Draft {
  id: string
  forum_id: string | null
  topic_id: string | null
  title: string | null
  body_md: string
  updated_at: string
}

export interface Tag { id: string; name: string; slug: string; color: string; topic_count: number; created_at: string }

export interface PollOptionResult { id: string; text: string; votes: number; me: boolean }
export interface PollResults {
  poll: { id: string; topic_id: string; question: string; is_multiple: boolean; closes_at: string | null; created_at: string }
  options: PollOptionResult[]
  total_voters: number
  has_voted: boolean
  is_closed: boolean
}
export interface NewPoll { question: string; is_multiple: boolean; closes_at?: string | null; options: string[] }

export interface ForumStats {
  categories: number; forums: number; topics: number; posts: number
  members: number; reactions: number; online: number; latest_member: string | null
}
export interface ModLogEntry {
  id: number; moderator_id: string; action: string
  forum_id: string | null; topic_id: string | null; post_id: string | null
  target_user_id: string | null; details: string | null; created_at: string
}
export interface Warning { id: string; user_id: string; moderator_id: string; reason: string; created_at: string }
export interface Ban { user_id: string; banned_by: string; reason: string | null; until: string | null; created_at: string }
// IP / email bans (exact match, admin only — see services::ban_registry).
export interface IpBan { id: string; value: string; reason: string | null; banned_by: string; until: string | null; created_at: string }
export interface EmailBan { id: string; email: string; reason: string | null; banned_by: string; until: string | null; created_at: string }

// Private messages (modern conversations, distinct from the legacy Post/Topic model)
export interface PmThreadSummary {
  id: string
  subject: string | null
  last_message_at: string
  last_message_preview: string
  unread_count: number
  participant_ids: string[]
}
export interface PmThread {
  id: string
  subject: string | null
  participant_ids: string[]
}
export interface PmMessage {
  id: string
  sender_id: string
  body_md: string
  created_at: string
}
export interface PmThreadDetail {
  thread: PmThread
  messages: PmMessage[]
}

// Ignore list — a purely client-side display preference:
// the backend keeps returning every post from an ignored member, the
// frontend just folds them by default (`GET /me/ignored` returns bare ids).

// ── Client ────────────────────────────────────────────────────────────────────

const qs = (q: Record<string, unknown>) => {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(q)) {
    if (v !== undefined && v !== null && v !== '') p.set(k, String(v))
  }
  const s = p.toString()
  return s ? `?${s}` : ''
}

/** A personal RSS/Atom feed credential (its `token` appears in the feed URL). */
export interface FeedToken {
  token: string
  user_id: string
  label: string | null
  created_at: string
  last_used_at: string | null
}

export const forumApi = {
  // Categories
  listCategories: () =>
    apiClient.get<{ categories: Category[] }>('/forum/categories').then(r => r.data.categories),
  createCategory: (body: { name: string; description?: string; position?: number }) =>
    apiClient.post<{ category: Category }>('/forum/categories', body).then(r => r.data.category),
  updateCategory: (id: string, body: Partial<Category>) =>
    apiClient.patch<{ category: Category }>(`/forum/categories/${id}`, body).then(r => r.data.category),
  deleteCategory: (id: string) => apiClient.delete(`/forum/categories/${id}`).then(() => undefined),

  // Forums
  listForums: (categoryId?: string) =>
    apiClient.get<{ forums: Forum[] }>(`/forum/forums${qs({ category_id: categoryId })}`).then(r => r.data.forums),
  getForum: (id: string) =>
    apiClient.get<{ forum: Forum; permissions: ForumPerms }>(`/forum/forums/${id}`).then(r => r.data),
  createForum: (body: { category_id: string; parent_forum_id?: string | null; name: string; description?: string; position?: number }) =>
    apiClient.post<{ forum: Forum }>('/forum/forums', body).then(r => r.data.forum),
  updateForum: (id: string, body: Partial<Forum>) =>
    apiClient.patch<{ forum: Forum }>(`/forum/forums/${id}`, body).then(r => r.data.forum),
  deleteForum: (id: string) => apiClient.delete(`/forum/forums/${id}`).then(() => undefined),
  reorderForums: (ids: string[]) => apiClient.patch('/forum/forums/reorder', { ids }).then(() => undefined),
  pruneForum: (forumId: string, body: { days: number; dry_run: boolean }) =>
    apiClient.post<{ count: number }>(`/forum/forums/${forumId}/prune`, body).then(r => r.data.count),
  forumReadState: (id: string) =>
    apiClient.get<{ read_state: ReadState[] }>(`/forum/forums/${id}/read-state`).then(r => r.data.read_state),
  subscribeForum: (id: string) => apiClient.post(`/forum/forums/${id}/subscribe`, {}).then(() => undefined),
  unsubscribeForum: (id: string) => apiClient.delete(`/forum/forums/${id}/subscribe`).then(() => undefined),
  markForumRead: (forumId: string) => apiClient.post(`/forum/forums/${forumId}/read-all`).then(() => undefined),
  markAllRead: () => apiClient.post('/forum/me/read-all').then(() => undefined),

  // Topics
  listTopics: (forumId: string, q: { limit?: number; offset?: number } = {}) =>
    apiClient.get<{ topics: Topic[]; total: number }>(`/forum/forums/${forumId}/topics${qs(q)}`).then(r => r.data),
  createTopic: (forumId: string, body: { title: string; body_md: string; topic_type?: TopicType; is_question?: boolean; prefix?: string; tag_ids?: string[]; poll?: NewPoll }) =>
    apiClient.post<{ topic: Topic; post: Post }>(`/forum/forums/${forumId}/topics`, body).then(r => r.data),
  getTopic: (id: string) =>
    apiClient.get<{ topic: Topic; permissions: ForumPerms }>(`/forum/topics/${id}`).then(r => r.data),
  updateTopic: (id: string, body: { title?: string; topic_type?: TopicType; is_locked?: boolean }) =>
    apiClient.patch<{ topic: Topic }>(`/forum/topics/${id}`, body).then(r => r.data.topic),
  deleteTopic: (id: string) => apiClient.delete(`/forum/topics/${id}`).then(() => undefined),
  lockTopic: (id: string, locked: boolean) =>
    apiClient.post<{ topic: Topic }>(`/forum/topics/${id}/${locked ? 'lock' : 'unlock'}`, {}).then(r => r.data.topic),
  moveTopic: (id: string, forumId: string) =>
    apiClient.post<{ topic: Topic }>(`/forum/topics/${id}/move`, { forum_id: forumId }).then(r => r.data.topic),
  splitTopic: (id: string, body: { post_ids: string[]; title: string; forum_id?: string }) =>
    apiClient.post<{ topic: Topic }>(`/forum/topics/${id}/split`, body).then(r => r.data.topic),
  mergeTopic: (id: string, sourceTopicId: string) =>
    apiClient.post<{ topic: Topic }>(`/forum/topics/${id}/merge`, { source_topic_id: sourceTopicId }).then(r => r.data.topic),
  markRead: (id: string, lastReadPostId?: string | null) =>
    apiClient.post(`/forum/topics/${id}/read`, { last_read_post_id: lastReadPostId ?? null }).then(() => undefined),
  // This user's own read marker for the topic ("jump to first unread post").
  topicReadState: (id: string) =>
    apiClient.get<{ read_at: string | null }>(`/forum/topics/${id}/read-state`).then(r => r.data.read_at),
  subscribeTopic: (id: string) => apiClient.post(`/forum/topics/${id}/subscribe`, {}).then(() => undefined),
  unsubscribeTopic: (id: string) => apiClient.delete(`/forum/topics/${id}/subscribe`).then(() => undefined),

  // Posts
  listPosts: (topicId: string, q: { limit?: number; offset?: number } = {}) =>
    apiClient.get<{ posts: Post[]; total: number }>(`/forum/topics/${topicId}/posts${qs(q)}`).then(r => r.data),
  getPost: (id: string) =>
    apiClient.get<{ post: Post }>(`/forum/posts/${id}`).then(r => r.data.post),
  createPost: (topicId: string, body: { body_md: string; reply_to_post_id?: string | null; mention_user_ids?: string[] }) =>
    apiClient.post<{ post: Post }>(`/forum/topics/${topicId}/posts`, body).then(r => r.data.post),
  updatePost: (id: string, body: { body_md: string; edit_reason?: string }) =>
    apiClient.patch<{ post: Post }>(`/forum/posts/${id}`, body).then(r => r.data.post),
  deletePost: (id: string) => apiClient.delete(`/forum/posts/${id}`).then(() => undefined),
  getPostRevisions: (id: string) =>
    apiClient.get<{ revisions: PostRevision[] }>(`/forum/posts/${id}/revisions`).then(r => r.data.revisions),
  reportPost: (id: string, body: { reason: string; reason_id?: string | null }) =>
    apiClient.post<{ report: Report }>(`/forum/posts/${id}/report`, body).then(r => r.data.report),

  // Attachments
  listAttachments: (postId: string) =>
    apiClient.get<{ attachments: Attachment[] }>(`/forum/posts/${postId}/attachments`).then(r => r.data.attachments),
  createAttachment: (postId: string, body: { file_id?: string | null; filename: string; mime_type?: string; size_bytes?: number }) =>
    apiClient.post<{ attachment: Attachment }>(`/forum/posts/${postId}/attachments`, body).then(r => r.data.attachment),
  deleteAttachment: (id: string) => apiClient.delete(`/forum/attachments/${id}`).then(() => undefined),

  // Moderation
  listReports: (status?: string) =>
    apiClient.get<{ reports: Report[] }>(`/forum/reports${qs({ status })}`).then(r => r.data.reports),
  listReportReasons: () =>
    apiClient.get<{ reasons: ReportReason[] }>('/forum/report-reasons').then(r => r.data.reasons),
  createReportReason: (body: { title: string; description?: string | null; position?: number }) =>
    apiClient.post<{ reason: ReportReason }>('/forum/report-reasons', body).then(r => r.data.reason),
  deleteReportReason: (id: string) => apiClient.delete(`/forum/report-reasons/${id}`).then(() => undefined),
  listCensoredWords: () =>
    apiClient.get<{ words: CensoredWord[] }>('/forum/censored-words').then(r => r.data.words),
  createCensoredWord: (body: { pattern: string; replacement?: string | null }) =>
    apiClient.post<{ word: CensoredWord }>('/forum/censored-words', body).then(r => r.data.word),
  deleteCensoredWord: (id: string) => apiClient.delete(`/forum/censored-words/${id}`).then(() => undefined),

  // FAQ (admin-curated, read by every member)
  listFaq: () =>
    apiClient.get<{ entries: FaqEntry[] }>('/forum/faq').then(r => r.data.entries),
  createFaq: (body: { question: string; answer_md: string; position?: number }) =>
    apiClient.post<{ entry: FaqEntry }>('/forum/faq', body).then(r => r.data.entry),
  updateFaq: (id: string, body: { question?: string; answer_md?: string; position?: number }) =>
    apiClient.patch<{ entry: FaqEntry }>(`/forum/faq/${id}`, body).then(r => r.data.entry),
  deleteFaq: (id: string) => apiClient.delete(`/forum/faq/${id}`).then(() => undefined),

  // Custom profile fields (definitions, admin-curated)
  listProfileFields: () =>
    apiClient.get<{ fields: ProfileField[] }>('/forum/profile-fields').then(r => r.data.fields),
  createProfileField: (body: {
    key: string; label: string; field_type: ProfileFieldType; options?: string[] | null
    position?: number; visibility?: ProfileFieldVisibility; show_on_posts?: boolean; required?: boolean
  }) =>
    apiClient.post<{ field: ProfileField }>('/forum/profile-fields', body).then(r => r.data.field),
  updateProfileField: (id: string, body: {
    key?: string; label?: string; field_type?: ProfileFieldType; options?: string[] | null
    position?: number; visibility?: ProfileFieldVisibility; show_on_posts?: boolean; required?: boolean
  }) =>
    apiClient.patch<{ field: ProfileField }>(`/forum/profile-fields/${id}`, body).then(r => r.data.field),
  deleteProfileField: (id: string) => apiClient.delete(`/forum/profile-fields/${id}`).then(() => undefined),
  resolveReport: (id: string, status: 'resolved' | 'rejected') =>
    apiClient.patch<{ report: Report }>(`/forum/reports/${id}`, { status }).then(r => r.data.report),
  pendingQueue: () =>
    apiClient.get<{ pending: PendingPost[]; total: number }>('/forum/mod/queue').then(r => r.data),
  approvePending: (id: string) =>
    apiClient.post(`/forum/mod/queue/${id}/approve`).then(() => undefined),
  rejectPending: (id: string) =>
    apiClient.post(`/forum/mod/queue/${id}/reject`).then(() => undefined),
  listModerators: (forumId: string) =>
    apiClient.get<{ moderators: Moderator[] }>(`/forum/forums/${forumId}/moderators`).then(r => r.data.moderators),
  addModerator: (forumId: string, userId: string) =>
    apiClient.post<{ moderator: Moderator }>(`/forum/forums/${forumId}/moderators`, { user_id: userId }).then(r => r.data.moderator),
  removeModerator: (forumId: string, userId: string) =>
    apiClient.delete(`/forum/forums/${forumId}/moderators/${userId}`).then(() => undefined),

  // Permissions
  listPermissions: (forumId: string) =>
    apiClient.get<{ permissions: Permission[] }>(`/forum/forums/${forumId}/permissions`).then(r => r.data.permissions),
  setPermission: (forumId: string, body: Omit<Permission, 'id' | 'forum_id'>) =>
    apiClient.put<{ permission: Permission }>(`/forum/forums/${forumId}/permissions`, body).then(r => r.data.permission),

  // Group permissions (additive grants that can open a restricted forum to a group)
  listGroups: () =>
    apiClient.get<{ groups: DirectoryGroup[] }>('/forum/groups').then(r => r.data.groups),
  listGroupPermissions: (forumId: string) =>
    apiClient.get<{ permissions: GroupPermission[] }>(`/forum/forums/${forumId}/group-permissions`).then(r => r.data.permissions),
  setGroupPermission: (forumId: string, body: { group_id: string; can_view: boolean; can_post: boolean; can_reply: boolean; can_attach: boolean }) =>
    apiClient.put<{ permissions: GroupPermission[] }>(`/forum/forums/${forumId}/group-permissions`, body).then(r => r.data.permissions),

  // Ranks & profiles
  listRanks: () => apiClient.get<{ ranks: Rank[] }>('/forum/ranks').then(r => r.data.ranks),
  createRank: (body: { title: string; min_posts?: number; is_special?: boolean; badge?: string }) =>
    apiClient.post<{ rank: Rank }>('/forum/ranks', body).then(r => r.data.rank),
  updateRank: (id: string, body: Partial<Rank>) =>
    apiClient.patch<{ rank: Rank }>(`/forum/ranks/${id}`, body).then(r => r.data.rank),
  deleteRank: (id: string) => apiClient.delete(`/forum/ranks/${id}`).then(() => undefined),
  assignRank: (uid: string, rankId: string | null) =>
    apiClient.patch<{ profile: UserProfile }>(`/forum/profiles/${uid}/rank`, { rank_id: rankId }).then(r => r.data.profile),
  getProfile: (uid: string) =>
    apiClient.get<{ profile: UserProfile }>(`/forum/profiles/${uid}`).then(r => r.data.profile),
  getProfilePage: (uid: string) =>
    apiClient.get<{ profile: UserProfile; topics: Topic[] }>(`/forum/profiles/${uid}`).then(r => r.data),
  getBriefProfiles: (ids: string[]) =>
    ids.length === 0
      ? Promise.resolve([] as BriefProfile[])
      : apiClient.get<{ profiles: BriefProfile[] }>(`/forum/profiles/brief${qs({ ids: ids.join(',') })}`).then(r => r.data.profiles),
  myProfile: () => apiClient.get<{ profile: UserProfile }>('/forum/me/profile').then(r => r.data.profile),
  updateMySignature: (signature_md: string | null) =>
    apiClient.patch<{ profile: UserProfile }>('/forum/me/profile', { signature_md }).then(r => r.data.profile),
  mySubscriptions: () =>
    apiClient.get<{ subscriptions: Subscription[] }>('/forum/me/subscriptions').then(r => r.data.subscriptions),

  // Search
  search: (
    q: string,
    opts: {
      limit?: number
      offset?: number
      /** Restrict to posts authored by this user. */
      author_id?: string
      /** Restrict to these forums. */
      forum_ids?: string[]
      /** 'all' (default) | 'title' | 'body'. */
      scope?: 'all' | 'title' | 'body'
      /** 'relevance' (default) | 'recent'. */
      sort?: 'relevance' | 'recent'
      /** Restrict to posts created within the last N days. */
      days?: number
    } = {},
  ) => {
    const { forum_ids, ...rest } = opts
    return apiClient
      .get<{ results: SearchHit[]; total: number }>(
        `/forum/search${qs({ q, ...rest, forum_ids: forum_ids?.length ? forum_ids.join(',') : undefined })}`,
      )
      .then(r => r.data)
  },

  // Reactions
  react: (postId: string, emoji: string) =>
    apiClient.post<{ added: boolean; reactions: EmojiAgg[] }>(`/forum/posts/${postId}/react`, { emoji }).then(r => r.data),
  topicReactions: (topicId: string) =>
    apiClient.get<{ reactions: Record<string, EmojiAgg[]> }>(`/forum/topics/${topicId}/reactions`).then(r => r.data.reactions),
  // Who reacted to a post, grouped by emoji — loaded on demand (hover/click
  // on a reaction chip), never for every post of a topic at once.
  postReactionUsers: (postId: string) =>
    apiClient.get<{ reactions: ReactionUsers[] }>(`/forum/posts/${postId}/reactions/users`).then(r => r.data.reactions),

  // Solution
  setSolution: (topicId: string, postId: string) =>
    apiClient.post<{ topic: Topic }>(`/forum/topics/${topicId}/solution`, { post_id: postId }).then(r => r.data.topic),
  clearSolution: (topicId: string) =>
    apiClient.delete<{ topic: Topic }>(`/forum/topics/${topicId}/solution`).then(r => r.data.topic),

  // Bookmarks
  toggleBookmark: (topicId: string) =>
    apiClient.post<{ bookmarked: boolean }>(`/forum/topics/${topicId}/bookmark`, {}).then(r => r.data.bookmarked),
  listBookmarks: () =>
    apiClient.get<{ topics: Topic[] }>('/forum/me/bookmarks').then(r => r.data.topics),

  // Notifications
  listNotifications: (unread = false) =>
    apiClient.get<{ notifications: ForumNotification[]; unread: number }>('/forum/me/notifications', { params: { unread } }).then(r => r.data),
  markNotifications: (id?: string) =>
    apiClient.post<{ unread: number }>('/forum/me/notifications/read', { id: id ?? null }).then(r => r.data.unread),

  // Drafts
  saveDraft: (body: { forum_id?: string; topic_id?: string; title?: string; body_md: string }) =>
    apiClient.put<{ draft: Draft | null }>('/forum/me/drafts', body).then(r => r.data.draft),
  listDrafts: () => apiClient.get<{ drafts: Draft[] }>('/forum/me/drafts').then(r => r.data.drafts),
  deleteDraft: (id: string) => apiClient.delete(`/forum/me/drafts/${id}`).then(() => undefined),

  // Tags
  listTags: () => apiClient.get<{ tags: Tag[] }>('/forum/tags').then(r => r.data.tags),
  createTag: (name: string, color?: string) =>
    apiClient.post<{ tag: Tag }>('/forum/tags', { name, color }).then(r => r.data.tag),
  deleteTag: (id: string) => apiClient.delete(`/forum/tags/${id}`).then(() => undefined),
  topicTags: (topicId: string) => apiClient.get<{ tags: Tag[] }>(`/forum/topics/${topicId}/tags`).then(r => r.data.tags),
  setTopicTags: (topicId: string, tagIds: string[]) =>
    apiClient.put<{ tags: Tag[] }>(`/forum/topics/${topicId}/tags`, { tag_ids: tagIds }).then(r => r.data.tags),

  // Polls
  getPoll: (topicId: string) =>
    apiClient.get<{ poll: PollResults | null }>(`/forum/topics/${topicId}/poll`).then(r => r.data.poll),
  votePoll: (pollId: string, optionIds: string[]) =>
    apiClient.post<{ poll: PollResults }>(`/forum/polls/${pollId}/vote`, { option_ids: optionIds }).then(r => r.data.poll),

  // Discovery
  feed: (kind: string, opts: { solved?: boolean; tag?: string; limit?: number; offset?: number } = {}) =>
    apiClient.get<{ topics: Topic[]; tags: Record<string, Tag[]>; total: number }>(`/forum/feed${qs({ kind, ...opts })}`).then(r => r.data),

  // Community
  heartbeat: (path?: string) => apiClient.post('/forum/me/heartbeat', { path }).then(() => undefined),
  online: () => apiClient.get<{ user_ids: string[] }>('/forum/online').then(r => r.data.user_ids),
  whosOnlineDetailed: () => apiClient.get<{ users: OnlineUser[] }>('/forum/online/detailed').then(r => r.data.users),
  stats: () => apiClient.get<{ stats: ForumStats }>('/forum/stats').then(r => r.data.stats),
  leaderboard: () => apiClient.get<{ top: LeaderboardEntry[] }>('/forum/leaderboard').then(r => r.data.top),
  members: (opts: { sort?: string; limit?: number; offset?: number } = {}) =>
    apiClient.get<{ members: Member[]; total: number }>(`/forum/members${qs(opts)}`).then(r => r.data),
  team: () => apiClient.get<{ moderators: TeamMember[] }>('/forum/team').then(r => r.data.moderators),

  // Profiles
  profileActivity: (uid: string) => apiClient.get<{ posts: Post[] }>(`/forum/profiles/${uid}/activity`).then(r => r.data.posts),
  updateProfile: (body: { signature_md?: string; bio_md?: string; location?: string; website?: string; custom_title?: string }) =>
    apiClient.patch<{ profile: UserProfile }>('/forum/me/profile', body).then(r => r.data.profile),

  // Custom profile fields (values, per member)
  getMyProfileFields: () =>
    apiClient.get<{ fields: ProfileField[]; values: ProfileFieldValue[] }>('/forum/me/profile-fields').then(r => r.data),
  setMyProfileFields: (values: { field_id: string; value: string }[]) =>
    apiClient.put<{ values: ProfileFieldValue[] }>('/forum/me/profile-fields', { values }).then(r => r.data.values),
  getUserProfileFields: (uid: string) =>
    apiClient.get<{ fields: { field: ProfileField; value: string }[] }>(`/forum/users/${uid}/profile-fields`).then(r => r.data.fields),

  // Advanced moderation
  modLog: () => apiClient.get<{ log: ModLogEntry[] }>('/forum/mod/log').then(r => r.data.log),
  listBans: () => apiClient.get<{ bans: Ban[] }>('/forum/mod/bans').then(r => r.data.bans),
  warnUser: (uid: string, reason: string) =>
    apiClient.post<{ warning: Warning }>(`/forum/mod/users/${uid}/warn`, { reason }).then(r => r.data.warning),
  listWarnings: (uid: string) => apiClient.get<{ warnings: Warning[] }>(`/forum/mod/users/${uid}/warnings`).then(r => r.data.warnings),
  banUser: (uid: string, reason?: string, days?: number) =>
    apiClient.post<{ ban: Ban }>(`/forum/mod/users/${uid}/ban`, { reason, days }).then(r => r.data.ban),
  unbanUser: (uid: string) => apiClient.delete(`/forum/mod/users/${uid}/ban`).then(() => undefined),
  // IP / email bans (exact match, admin only).
  listIpBans: () => apiClient.get<{ bans: IpBan[] }>('/forum/bans/ip').then(r => r.data.bans),
  banIp: (value: string, reason?: string, days?: number) =>
    apiClient.post<{ ban: IpBan }>('/forum/bans/ip', { value, reason, days }).then(r => r.data.ban),
  unbanIp: (id: string) => apiClient.delete(`/forum/bans/ip/${id}`).then(() => undefined),
  listEmailBans: () => apiClient.get<{ bans: EmailBan[] }>('/forum/bans/email').then(r => r.data.bans),
  banEmail: (email: string, reason?: string, days?: number) =>
    apiClient.post<{ ban: EmailBan }>('/forum/bans/email', { email, reason, days }).then(r => r.data.ban),
  unbanEmail: (id: string) => apiClient.delete(`/forum/bans/email/${id}`).then(() => undefined),
  removePost: (id: string) => apiClient.post(`/forum/posts/${id}/remove`, {}).then(() => undefined),
  restorePost: (id: string) => apiClient.post<{ ok: boolean }>(`/forum/posts/${id}/restore`, {}).then(() => undefined),

  bulkModerateTopics: (body: { topic_ids: string[]; action: 'lock' | 'unlock' | 'delete' }) =>
    apiClient.post<{ done: number; skipped: number }>('/forum/mod/topics/bulk', body).then(r => r.data),

  // Trash (soft-deleted topics)
  listTrash: () => apiClient.get<{ topics: Topic[] }>('/forum/trash').then(r => r.data.topics),
  restoreTopic: (id: string) => apiClient.post<{ topic: Topic }>(`/forum/topics/${id}/restore`, {}).then(r => r.data.topic),
  purgeTopic: (id: string) => apiClient.delete(`/forum/topics/${id}/purge`).then(() => undefined),

  // Users (core directory)
  searchUsers: (q: string) =>
    apiClient.get<{ users: UserBrief[] }>('/users/search', { params: { q, limit: 8 } }).then(r => r.data.users),
  lookupUsers: (ids: string[]) =>
    ids.length === 0
      ? Promise.resolve([] as UserBrief[])
      : apiClient.get<{ users: UserBrief[] }>('/users/lookup', { params: { ids: ids.join(',') } }).then(r => r.data.users),

  // Private messages
  listPmThreads: () =>
    apiClient.get<{ threads: PmThreadSummary[] }>('/forum/me/pm').then(r => r.data.threads),
  createPmThread: (body: { recipient_ids: string[]; subject?: string; body_md: string }) =>
    apiClient.post<{ thread: PmThread; message: PmMessage }>('/forum/me/pm', body).then(r => r.data),
  getPmThread: (id: string) =>
    apiClient.get<PmThreadDetail>(`/forum/me/pm/${id}`).then(r => r.data),
  sendPmMessage: (id: string, body_md: string) =>
    apiClient.post<{ message: PmMessage }>(`/forum/me/pm/${id}`, { body_md }).then(r => r.data.message),
  markPmRead: (id: string) => apiClient.post(`/forum/me/pm/${id}/read`, {}).then(() => undefined),
  deletePmThread: (id: string) => apiClient.delete(`/forum/me/pm/${id}`).then(() => undefined),
  pmUnreadCount: () => apiClient.get<{ count: number }>('/forum/me/pm/unread').then(r => r.data.count),
  listPmBlocks: () => apiClient.get<{ blocked: string[] }>('/forum/me/pm/blocks').then(r => r.data.blocked),
  blockPmUser: (user_id: string) => apiClient.post('/forum/me/pm/blocks', { user_id }).then(() => undefined),
  unblockPmUser: (uid: string) => apiClient.delete(`/forum/me/pm/blocks/${uid}`).then(() => undefined),

  // Ignore list — display-only, see the note above.
  listIgnored: () => apiClient.get<{ ignored: string[] }>('/forum/me/ignored').then(r => r.data.ignored),
  ignoreUser: (user_id: string) => apiClient.post('/forum/me/ignored', { user_id }).then(() => undefined),
  unignoreUser: (uid: string) => apiClient.delete(`/forum/me/ignored/${uid}`).then(() => undefined),

  // Personal RSS/Atom feed tokens. The feed documents are served anonymously at
  // `/api/v1/forum/public/feeds/:token/{atom,rss}.xml` (see FeedsPage).
  listFeedTokens: () => apiClient.get<{ tokens: FeedToken[] }>('/forum/me/feed-tokens').then(r => r.data.tokens),
  createFeedToken: (label?: string) =>
    apiClient.post<{ token: FeedToken }>('/forum/me/feed-tokens', { label: label || null }).then(r => r.data.token),
  revokeFeedToken: (token: string) =>
    apiClient.delete(`/forum/me/feed-tokens/${encodeURIComponent(token)}`).then(() => undefined),
}
