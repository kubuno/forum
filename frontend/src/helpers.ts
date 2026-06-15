/** Relative "time ago" using the platform locale, falling back to a short date. */
export function timeAgo(iso: string | null): string {
  if (!iso) return ''
  const then = new Date(iso).getTime()
  const diff = Date.now() - then
  const sec = Math.round(diff / 1000)
  const min = Math.round(sec / 60)
  const hr = Math.round(min / 60)
  const day = Math.round(hr / 24)
  const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })
  if (sec < 60) return rtf.format(-sec, 'second')
  if (min < 60) return rtf.format(-min, 'minute')
  if (hr < 24) return rtf.format(-hr, 'hour')
  if (day < 7) return rtf.format(-day, 'day')
  return new Date(iso).toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' })
}

export function shortDateTime(iso: string | null): string {
  if (!iso) return ''
  return new Date(iso).toLocaleString(undefined, {
    day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit',
  })
}

export function formatBytes(n: number | null | undefined): string {
  if (!n && n !== 0) return ''
  if (n < 1024) return `${n} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let v = n / 1024
  let i = 0
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++ }
  return `${v.toFixed(1)} ${units[i]}`
}
