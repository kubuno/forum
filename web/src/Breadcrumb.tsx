import { Link } from 'react-router-dom'
import { ChevronRight } from 'lucide-react'

export interface Crumb {
  label: string
  /** A route to link to. The last crumb is always rendered inert. */
  to?: string
}

/**
 * Navigation trail (Category › Forum › Sub-forum › Topic). Every crumb but the
 * last is a real anchor, so it is middle-clickable and shows its target on
 * hover; the last one is the current location and stays inert.
 */
export default function Breadcrumb({ items }: { items: Crumb[] }) {
  return (
    <nav aria-label="breadcrumb" className="flex items-center gap-1 text-xs text-text-tertiary mb-3 flex-wrap">
      {items.map((c, i) => {
        const isLast = i === items.length - 1
        return (
          <span key={`${c.label}-${i}`} className="flex items-center gap-1 min-w-0">
            {i > 0 && <ChevronRight size={12} className="shrink-0" />}
            {c.to && !isLast ? (
              <Link to={c.to} className="hover:text-text-primary truncate max-w-[16rem]">{c.label}</Link>
            ) : (
              <span className={`truncate max-w-[16rem] ${isLast ? 'text-text-secondary' : ''}`}>{c.label}</span>
            )}
          </span>
        )
      })}
    </nav>
  )
}
