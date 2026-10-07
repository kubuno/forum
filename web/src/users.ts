import { useEffect } from 'react'
import { create } from 'zustand'
import { forumApi, type UserBrief } from './api'

interface UsersState {
  cache: Record<string, UserBrief>
  pending: Set<string>
  ingest: (users: UserBrief[]) => void
  markPending: (ids: string[]) => string[]
}

const useUsersStore = create<UsersState>((set, get) => ({
  cache: {},
  pending: new Set(),
  ingest: (users) =>
    set((s) => {
      const cache = { ...s.cache }
      const pending = new Set(s.pending)
      for (const u of users) { cache[u.id] = u; pending.delete(u.id) }
      return { cache, pending }
    }),
  // Returns the ids that are not yet cached or in flight, and marks them pending.
  markPending: (ids) => {
    const { cache, pending } = get()
    const fresh = ids.filter((id) => id && !cache[id] && !pending.has(id))
    if (fresh.length) {
      const next = new Set(pending)
      fresh.forEach((id) => next.add(id))
      set({ pending: next })
    }
    return fresh
  },
}))

/** Ensure the given user ids are resolved into the shared cache. */
export function useResolveUsers(ids: (string | null | undefined)[]) {
  const ingest = useUsersStore((s) => s.ingest)
  const markPending = useUsersStore((s) => s.markPending)
  const key = ids.filter(Boolean).join(',')

  useEffect(() => {
    const wanted = key ? key.split(',') : []
    const fresh = markPending(wanted)
    if (!fresh.length) return
    let cancelled = false
    forumApi.lookupUsers(fresh).then((users) => { if (!cancelled) ingest(users) }).catch(() => {})
    return () => { cancelled = true }
  }, [key, ingest, markPending])
}

/** Read a resolved user (or undefined while loading). */
export function useUser(id: string | null | undefined): UserBrief | undefined {
  return useUsersStore((s) => (id ? s.cache[id] : undefined))
}

export function userLabel(u: UserBrief | undefined, fallback = '…'): string {
  return u?.display_name || u?.username || fallback
}
