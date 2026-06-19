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
  edited_at: string | null
  edited_by: string | null
  edit_reason: string | null
  edit_count: number
  is_deleted: boolean
  like_count: number
  created_at: string
  updated_at: string
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
  status: 'open' | 'resolved' | 'rejected'
  handled_by: string | null
  handled_at: string | null
  created_at: string
}

export interface Moderator {
  forum_id: string
  user_id: string
  created_at: string
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

export interface Permission {
  id: string
  forum_id: string
  role: 'guest' | 'user' | 'moderator'
  can_view: boolean
  can_post: boolean
  can_reply: boolean
  can_attach: boolean
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

export interface ForumNotification {
  id: string
  kind: string
  actor_id: string | null
  topic_id: string | null
  post_id: string | null
  extra: string | null
  is_read: boolean
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

// ── Client ────────────────────────────────────────────────────────────────────

const qs = (q: Record<string, unknown>) => {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(q)) {
    if (v !== undefined && v !== null && v !== '') p.set(k, String(v))
  }
  const s = p.toString()
  return s ? `?${s}` : ''
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
  forumReadState: (id: string) =>
    apiClient.get<{ read_state: ReadState[] }>(`/forum/forums/${id}/read-state`).then(r => r.data.read_state),
  subscribeForum: (id: string) => apiClient.post(`/forum/forums/${id}/subscribe`, {}).then(() => undefined),
  unsubscribeForum: (id: string) => apiClient.delete(`/forum/forums/${id}/subscribe`).then(() => undefined),

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
  reportPost: (id: string, reason: string) =>
    apiClient.post<{ report: Report }>(`/forum/posts/${id}/report`, { reason }).then(r => r.data.report),

  // Attachments
  listAttachments: (postId: string) =>
    apiClient.get<{ attachments: Attachment[] }>(`/forum/posts/${postId}/attachments`).then(r => r.data.attachments),
  createAttachment: (postId: string, body: { file_id?: string | null; filename: string; mime_type?: string; size_bytes?: number }) =>
    apiClient.post<{ attachment: Attachment }>(`/forum/posts/${postId}/attachments`, body).then(r => r.data.attachment),
  deleteAttachment: (id: string) => apiClient.delete(`/forum/attachments/${id}`).then(() => undefined),

  // Moderation
  listReports: (status?: string) =>
    apiClient.get<{ reports: Report[] }>(`/forum/reports${qs({ status })}`).then(r => r.data.reports),
  resolveReport: (id: string, status: 'resolved' | 'rejected') =>
    apiClient.patch<{ report: Report }>(`/forum/reports/${id}`, { status }).then(r => r.data.report),
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

  // Ranks & profiles
  listRanks: () => apiClient.get<{ ranks: Rank[] }>('/forum/ranks').then(r => r.data.ranks),
  createRank: (body: { title: string; min_posts?: number; is_special?: boolean; badge?: string }) =>
    apiClient.post<{ rank: Rank }>('/forum/ranks', body).then(r => r.data.rank),
  updateRank: (id: string, body: Partial<Rank>) =>
    apiClient.patch<{ rank: Rank }>(`/forum/ranks/${id}`, body).then(r => r.data.rank),
  deleteRank: (id: string) => apiClient.delete(`/forum/ranks/${id}`).then(() => undefined),
  getProfile: (uid: string) =>
    apiClient.get<{ profile: UserProfile }>(`/forum/profiles/${uid}`).then(r => r.data.profile),
  myProfile: () => apiClient.get<{ profile: UserProfile }>('/forum/me/profile').then(r => r.data.profile),
  updateMySignature: (signature_md: string | null) =>
    apiClient.patch<{ profile: UserProfile }>('/forum/me/profile', { signature_md }).then(r => r.data.profile),
  mySubscriptions: () =>
    apiClient.get<{ subscriptions: Subscription[] }>('/forum/me/subscriptions').then(r => r.data.subscriptions),

  // Search
  search: (q: string, opts: { limit?: number; offset?: number } = {}) =>
    apiClient.get<{ results: SearchHit[] }>(`/forum/search${qs({ q, ...opts })}`).then(r => r.data.results),

  // Reactions
  react: (postId: string, emoji: string) =>
    apiClient.post<{ added: boolean; reactions: EmojiAgg[] }>(`/forum/posts/${postId}/react`, { emoji }).then(r => r.data),
  topicReactions: (topicId: string) =>
    apiClient.get<{ reactions: Record<string, EmojiAgg[]> }>(`/forum/topics/${topicId}/reactions`).then(r => r.data.reactions),

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
    apiClient.get<{ topics: Topic[]; tags: Record<string, Tag[]> }>(`/forum/feed${qs({ kind, ...opts })}`).then(r => r.data),

  // Community
  heartbeat: (path?: string) => apiClient.post('/forum/me/heartbeat', { path }).then(() => undefined),
  online: () => apiClient.get<{ user_ids: string[] }>('/forum/online').then(r => r.data.user_ids),
  stats: () => apiClient.get<{ stats: ForumStats }>('/forum/stats').then(r => r.data.stats),
  members: () => apiClient.get<{ latest: string[]; top: { user_id: string; post_count: number }[] }>('/forum/members').then(r => r.data),

  // Profiles
  profileActivity: (uid: string) => apiClient.get<{ posts: Post[] }>(`/forum/profiles/${uid}/activity`).then(r => r.data.posts),
  updateProfile: (body: { signature_md?: string; bio_md?: string; location?: string; website?: string; custom_title?: string }) =>
    apiClient.patch<{ profile: UserProfile }>('/forum/me/profile', body).then(r => r.data.profile),

  // Advanced moderation
  modLog: () => apiClient.get<{ log: ModLogEntry[] }>('/forum/mod/log').then(r => r.data.log),
  listBans: () => apiClient.get<{ bans: Ban[] }>('/forum/mod/bans').then(r => r.data.bans),
  warnUser: (uid: string, reason: string) =>
    apiClient.post<{ warning: Warning }>(`/forum/mod/users/${uid}/warn`, { reason }).then(r => r.data.warning),
  listWarnings: (uid: string) => apiClient.get<{ warnings: Warning[] }>(`/forum/mod/users/${uid}/warnings`).then(r => r.data.warnings),
  banUser: (uid: string, reason?: string, days?: number) =>
    apiClient.post<{ ban: Ban }>(`/forum/mod/users/${uid}/ban`, { reason, days }).then(r => r.data.ban),
  unbanUser: (uid: string) => apiClient.delete(`/forum/mod/users/${uid}/ban`).then(() => undefined),
  removePost: (id: string) => apiClient.post(`/forum/posts/${id}/remove`, {}).then(() => undefined),
  restorePost: (id: string) => apiClient.post<{ ok: boolean }>(`/forum/posts/${id}/restore`, {}).then(() => undefined),

  // Users (core directory)
  searchUsers: (q: string) =>
    apiClient.get<{ users: UserBrief[] }>('/users/search', { params: { q, limit: 8 } }).then(r => r.data.users),
  lookupUsers: (ids: string[]) =>
    ids.length === 0
      ? Promise.resolve([] as UserBrief[])
      : apiClient.get<{ users: UserBrief[] }>('/users/lookup', { params: { ids: ids.join(',') } }).then(r => r.data.users),
}
