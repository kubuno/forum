import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNotificationStore } from '@kubuno/sdk'
import { forumApi, type ForumNotification, type UserBrief } from './api'
import { useResolveUsers, userLabel } from './users'

/** Mirrors the old (now orphaned) NotificationsBell.tsx label map. */
const KIND_LABEL: Record<string, string> = {
  reply: 'notif_reply', mention: 'notif_mention', reaction: 'notif_reaction',
  solution: 'notif_solution', quote: 'notif_quote', topic: 'notif_topic',
  report: 'notif_report', report_resolved: 'notif_report_resolved',
  approved: 'notif_approved', rejected: 'notif_rejected',
  warning: 'notif_warning', ban: 'notif_ban',
}

/**
 * Catch-up sync into the shared header bell.
 *
 * The forum has no bell of its own any more (see the orphaned
 * `NotificationsBell.tsx`) — everything the user needs to see lives in the
 * core's `useNotificationStore`. A real-time bridge already exists on the
 * core side (`useModuleNotifications`): the forum backend posts a `Custom`
 * WS event for every notification it creates, and that bridge turns it into
 * a bell entry keyed `forum:<notification_id>` (see
 * `NotificationService::push` in the forum's Rust backend).
 *
 * That bridge only fires while the browser is actually connected. This hook
 * covers what it misses: notifications created while the user was offline.
 * On mount, it reads the unread rows once and announces each one under the
 * SAME `forum:<id>` key — `pushKeyed` silently no-ops on a key already
 * announced, so anything the live bridge already delivered is never
 * duplicated, and remounting this hook (e.g. leaving and coming back to
 * /forum) never re-announces the same rows either.
 */
export function useForumNotificationSync(): void {
  const { t } = useTranslation('forum')
  const [unread, setUnread] = useState<ForumNotification[]>([])

  // Keeps the shared user cache warm for the resolved actors — other forum
  // UI (member lists, post authors…) then reuses it instead of refetching.
  useResolveUsers(unread.map(n => n.actor_id).filter(Boolean) as string[])

  useEffect(() => {
    let cancelled = false
    forumApi.listNotifications(true)
      .then(r => { if (!cancelled) setUnread(r.notifications) })
      .catch(() => { /* not logged in yet, or the forum backend is unreachable */ })
    return () => { cancelled = true }
  }, [])

  useEffect(() => {
    if (unread.length === 0) return
    let cancelled = false

    const actorIds = [...new Set(unread.map(n => n.actor_id).filter(Boolean))] as string[]
    const resolveActors = actorIds.length
      ? forumApi.lookupUsers(actorIds).catch(() => [] as UserBrief[])
      : Promise.resolve([] as UserBrief[])

    resolveActors.then(users => {
      if (cancelled) return
      const byId = new Map(users.map(u => [u.id, u]))
      const { pushKeyed } = useNotificationStore.getState()

      for (const n of unread) {
        const label = t(KIND_LABEL[n.kind] ?? 'notif_topic')
        const actor = n.actor_id ? byId.get(n.actor_id) : undefined
        // Actor name unavailable (lookup failed, or a system notification with
        // no actor, e.g. a ban/warning): fall back to the kind label alone.
        const others = n.responder_count > 1 ? ` ${t('notif_others', { count: n.responder_count - 1 })}` : ''
        const title = actor ? `${userLabel(actor)}${others} ${label}` : label

        pushKeyed(`forum:${n.id}`, {
          title,
          body: n.extra ?? '',
          moduleId: 'forum',
          icon: 'Bell',
          link: n.topic_id ? `/forum/topics/${n.topic_id}` : undefined,
        })
      }
    })

    return () => { cancelled = true }
  }, [unread, t])
}
