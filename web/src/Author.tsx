import { useUser, userLabel } from './users'

/** Inline author name (resolves via the shared user cache). */
export function AuthorName({ id, className }: { id: string | null; className?: string }) {
  const u = useUser(id)
  return <span className={className}>{userLabel(u)}</span>
}

/** Round avatar with initials fallback. */
export function AuthorAvatar({ id, size = 36 }: { id: string | null; size?: number }) {
  const u = useUser(id)
  const label = userLabel(u, '?')
  const initials = label.slice(0, 2).toUpperCase()
  if (u?.avatar_url) {
    return <img src={u.avatar_url} alt={label} width={size} height={size}
      className="rounded-full object-cover shrink-0" style={{ width: size, height: size }} />
  }
  return (
    <div className="rounded-full bg-primary-light text-primary flex items-center justify-center shrink-0 font-semibold"
      style={{ width: size, height: size, fontSize: size * 0.38 }}>
      {initials}
    </div>
  )
}
