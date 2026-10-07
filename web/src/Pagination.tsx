import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronLeft, ChevronRight } from 'lucide-react'

interface PaginationProps {
  /** 1-based current page. */
  page: number
  pageSize: number
  total: number
  onPage: (page: number) => void
}

/**
 * Shared pager for every listing (topics, posts, search, feed). Renders nothing
 * while a single page holds everything, so short lists stay clean. The page
 * window keeps the first, last and neighbouring pages visible with ellipses in
 * between, so it stays compact even across hundreds of pages.
 */
export default function Pagination({ page, pageSize, total, onPage }: PaginationProps) {
  const { t } = useTranslation('forum')
  const pages = Math.max(1, Math.ceil(total / pageSize))

  const items = useMemo(() => pageWindow(page, pages), [page, pages])
  if (pages <= 1) return null

  const go = (p: number) => { if (p >= 1 && p <= pages && p !== page) onPage(p) }

  return (
    <nav className="flex items-center justify-center gap-1 py-4" aria-label={t('page_indicator', { page, pages })}>
      <button
        type="button"
        onClick={() => go(page - 1)}
        disabled={page <= 1}
        aria-label={t('prev_page')}
        className="h-8 px-2 inline-flex items-center rounded-lg text-text-secondary hover:bg-surface-1 disabled:opacity-40 disabled:pointer-events-none"
      >
        <ChevronLeft size={16} />
      </button>

      {items.map((it, i) =>
        it === 'gap' ? (
          <span key={`gap-${i}`} className="h-8 px-1 inline-flex items-center text-text-tertiary select-none">…</span>
        ) : (
          <button
            key={it}
            type="button"
            onClick={() => go(it)}
            aria-current={it === page ? 'page' : undefined}
            className={`h-8 min-w-8 px-2 inline-flex items-center justify-center rounded-lg text-sm tabular-nums ${
              it === page
                ? 'bg-primary text-white font-medium'
                : 'text-text-secondary hover:bg-surface-1'
            }`}
          >
            {it}
          </button>
        ),
      )}

      <button
        type="button"
        onClick={() => go(page + 1)}
        disabled={page >= pages}
        aria-label={t('next_page')}
        className="h-8 px-2 inline-flex items-center rounded-lg text-text-secondary hover:bg-surface-1 disabled:opacity-40 disabled:pointer-events-none"
      >
        <ChevronRight size={16} />
      </button>
    </nav>
  )
}

/** First page, last page, current ± 1, with 'gap' markers where pages are skipped. */
function pageWindow(page: number, pages: number): (number | 'gap')[] {
  const set = new Set<number>([1, pages, page, page - 1, page + 1])
  const shown = [...set].filter(p => p >= 1 && p <= pages).sort((a, b) => a - b)
  const out: (number | 'gap')[] = []
  let prev = 0
  for (const p of shown) {
    if (prev && p - prev > 1) out.push('gap')
    out.push(p)
    prev = p
  }
  return out
}
