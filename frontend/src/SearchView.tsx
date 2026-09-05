import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'
import { useQuery, keepPreviousData } from '@tanstack/react-query'
import { Search, SlidersHorizontal, X } from 'lucide-react'
import { Input, Spinner, Dropdown, Checkbox } from '@ui'
import { forumApi, type UserBrief } from './api'
import { useForumStore } from './store'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'
import { timeAgo } from './helpers'
import Pagination from './Pagination'

const RESULTS_PER_PAGE = 20

type SearchScope = 'all' | 'title' | 'body'
type SearchSort = 'relevance' | 'recent'

/**
 * The one and only piece of state behind both the plain search bar and the
 * advanced panel — see the module-wide rule that they must never diverge.
 * The bar only ever touches `q`; the panel only ever touches the rest. Both
 * read from and write into this same object, so a filter set in the panel is
 * immediately what the bar's next search runs with, and vice versa.
 */
interface SearchState {
  q:            string
  authorId:     string | null
  authorLabel:  string | null
  forumIds:     string[]
  scope:        SearchScope
  sort:         SearchSort
  days:         number | null
}

const BASE_FILTERS: Omit<SearchState, 'q'> = {
  authorId: null, authorLabel: null, forumIds: [], scope: 'all', sort: 'relevance', days: null,
}

export default function SearchView() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const storeQuery = useForumStore(s => s.searchQuery)
  const setStoreQuery = useForumStore(s => s.setSearchQuery)

  const [search, setSearch] = useState<SearchState>(() => ({ q: storeQuery, ...BASE_FILTERS }))
  const [qDraft, setQDraft] = useState(search.q)
  const [panelOpen, setPanelOpen] = useState(false)
  const [page, setPage] = useState(1)

  const query = search.q.trim()
  const forumIdsKey = search.forumIds.join(',')

  // Any change to the shared query — from either the bar or the panel —
  // starts back at the first page.
  useEffect(() => { setPage(1) }, [query, search.authorId, forumIdsKey, search.scope, search.sort, search.days])

  const { data, isLoading } = useQuery({
    queryKey: ['forum-search', query, search.authorId, forumIdsKey, search.scope, search.sort, search.days, page],
    queryFn: () => forumApi.search(query, {
      limit:     RESULTS_PER_PAGE,
      offset:    (page - 1) * RESULTS_PER_PAGE,
      author_id: search.authorId ?? undefined,
      forum_ids: search.forumIds.length ? search.forumIds : undefined,
      scope:     search.scope !== 'all' ? search.scope : undefined,
      sort:      search.sort !== 'relevance' ? search.sort : undefined,
      days:      search.days ?? undefined,
    }),
    enabled: query.length > 1,
    placeholderData: keepPreviousData,
  })
  const results = data?.results ?? []
  const total = data?.total ?? 0
  useResolveUsers(results.map(r => r.author_id))

  const submitQuery = () => {
    setSearch(s => ({ ...s, q: qDraft }))
    setStoreQuery(qDraft)
  }

  const activeFilterCount =
    (search.authorId ? 1 : 0) +
    (search.forumIds.length > 0 ? 1 : 0) +
    (search.scope !== 'all' ? 1 : 0) +
    (search.sort !== 'relevance' ? 1 : 0) +
    (search.days ? 1 : 0)

  const clearAllFilters = () => setSearch(s => ({ ...s, ...BASE_FILTERS }))

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <form onSubmit={(e) => { e.preventDefault(); submitQuery() }} className="mb-3 flex items-center gap-2">
          <div className="flex-1">
            <Input value={qDraft} onChange={(e) => setQDraft(e.target.value)} autoFocus
              leftIcon={<Search size={16} />} placeholder={t('search_ph')} />
          </div>
          <button
            type="button"
            onClick={() => setPanelOpen(o => !o)}
            aria-expanded={panelOpen}
            className={`h-9 px-3 inline-flex items-center gap-1.5 rounded-lg border text-sm shrink-0 whitespace-nowrap ${
              panelOpen || activeFilterCount > 0
                ? 'border-primary text-primary bg-primary-light'
                : 'border-border text-text-secondary hover:bg-surface-1'
            }`}
          >
            <SlidersHorizontal size={14} />
            {t('advanced_search', { defaultValue: 'Recherche avancée' })}
            {activeFilterCount > 0 && <span className="font-semibold tabular-nums">{activeFilterCount}</span>}
          </button>
        </form>

        {panelOpen && (
          <div className="mb-4 rounded-xl border border-border bg-surface-0 p-4 grid gap-4 sm:grid-cols-2">
            <div>
              <label className="block text-xs font-medium text-text-secondary mb-1">
                {t('filter_author', { defaultValue: 'Auteur' })}
              </label>
              <AuthorFilterField
                authorId={search.authorId}
                authorLabel={search.authorLabel}
                onSelect={(u) => setSearch(s => ({ ...s, authorId: u.id, authorLabel: u.display_name || u.username }))}
                onClear={() => setSearch(s => ({ ...s, authorId: null, authorLabel: null }))}
                t={t}
              />
            </div>

            <div>
              <label className="block text-xs font-medium text-text-secondary mb-1">
                {t('filter_scope', { defaultValue: 'Portée' })}
              </label>
              <Dropdown
                value={search.scope}
                onChange={(v) => setSearch(s => ({ ...s, scope: v as SearchScope }))}
                options={[
                  { value: 'all',   label: t('scope_all',   { defaultValue: 'Titre et message' }) },
                  { value: 'title', label: t('scope_title', { defaultValue: 'Titre uniquement' }) },
                  { value: 'body',  label: t('scope_body',  { defaultValue: 'Message uniquement' }) },
                ]}
                width="100%" height={36}
              />
            </div>

            <div>
              <label className="block text-xs font-medium text-text-secondary mb-1">
                {t('filter_sort', { defaultValue: 'Trier par' })}
              </label>
              <Dropdown
                value={search.sort}
                onChange={(v) => setSearch(s => ({ ...s, sort: v as SearchSort }))}
                options={[
                  { value: 'relevance', label: t('sort_relevance', { defaultValue: 'Pertinence' }) },
                  { value: 'recent',    label: t('sort_recent',    { defaultValue: 'Plus récents' }) },
                ]}
                width="100%" height={36}
              />
            </div>

            <div>
              <label className="block text-xs font-medium text-text-secondary mb-1">
                {t('filter_period', { defaultValue: 'Période' })}
              </label>
              <Dropdown
                value={search.days ? String(search.days) : ''}
                onChange={(v) => setSearch(s => ({ ...s, days: v ? Number(v) : null }))}
                options={[
                  { value: '',    label: t('period_any',   { defaultValue: 'Toute période' }) },
                  { value: '1',   label: t('period_day',   { defaultValue: 'Dernières 24 h' }) },
                  { value: '7',   label: t('period_week',  { defaultValue: 'Dernière semaine' }) },
                  { value: '30',  label: t('period_month', { defaultValue: 'Dernier mois' }) },
                  { value: '365', label: t('period_year',  { defaultValue: 'Dernière année' }) },
                ]}
                width="100%" height={36}
              />
            </div>

            <div className="sm:col-span-2">
              <label className="block text-xs font-medium text-text-secondary mb-1">
                {t('filter_forums', { defaultValue: 'Forums' })}
              </label>
              <ForumFilterList
                selected={search.forumIds}
                onToggle={(id) => setSearch(s => ({
                  ...s,
                  forumIds: s.forumIds.includes(id) ? s.forumIds.filter(x => x !== id) : [...s.forumIds, id],
                }))}
                t={t}
              />
            </div>

            {activeFilterCount > 0 && (
              <div className="sm:col-span-2 flex justify-end">
                <button type="button" onClick={clearAllFilters}
                  className="text-xs text-text-secondary hover:text-text-primary underline">
                  {t('clear_filters', { defaultValue: 'Réinitialiser les filtres' })}
                </button>
              </div>
            )}
          </div>
        )}

        {activeFilterCount > 0 && (
          <div className="flex flex-wrap gap-1.5 mb-3">
            {search.authorId && (
              <FilterChip
                label={`${t('filter_author', { defaultValue: 'Auteur' })} : ${search.authorLabel}`}
                onRemove={() => setSearch(s => ({ ...s, authorId: null, authorLabel: null }))}
              />
            )}
            {search.forumIds.length > 0 && (
              <FilterChip
                label={t('filter_forums_count', { defaultValue: '{{n}} forum(s)', n: search.forumIds.length })}
                onRemove={() => setSearch(s => ({ ...s, forumIds: [] }))}
              />
            )}
            {search.scope !== 'all' && (
              <FilterChip
                label={search.scope === 'title'
                  ? t('scope_title', { defaultValue: 'Titre uniquement' })
                  : t('scope_body', { defaultValue: 'Message uniquement' })}
                onRemove={() => setSearch(s => ({ ...s, scope: 'all' }))}
              />
            )}
            {search.sort !== 'relevance' && (
              <FilterChip
                label={t('sort_recent', { defaultValue: 'Plus récents' })}
                onRemove={() => setSearch(s => ({ ...s, sort: 'relevance' }))}
              />
            )}
            {search.days && (
              <FilterChip
                label={t('filter_period_active', { defaultValue: 'Depuis {{n}} jours', n: search.days })}
                onRemove={() => setSearch(s => ({ ...s, days: null }))}
              />
            )}
          </div>
        )}

        <h1 className="text-sm font-semibold text-text-secondary mb-3">{t('search_results')}</h1>
        {isLoading ? (
          <div className="py-10 flex justify-center"><Spinner /></div>
        ) : query.length < 2 ? (
          <div className="py-10 text-center text-text-secondary">
            {t('search_prompt', { defaultValue: 'Saisissez un terme de recherche.' })}
          </div>
        ) : results.length === 0 ? (
          <div className="py-10 text-center text-text-secondary">{t('no_results')}</div>
        ) : (
          <ul className="rounded-xl border border-border overflow-hidden bg-surface-0 divide-y divide-border">
            {results.map(r => (
              <li key={r.post_id}>
                <button onClick={() => navigate(`/forum/topics/${r.topic_id}`)} className="w-full text-left px-4 py-3 hover:bg-surface-1">
                  <div className="font-medium text-text-primary truncate">{r.topic_title}</div>
                  <div className="text-sm text-text-secondary line-clamp-2">{renderSnippet(r.snippet)}</div>
                  <div className="text-xs text-text-tertiary mt-1">{t('by')} <AuthorName id={r.author_id} /> · {timeAgo(r.created_at)}</div>
                </button>
              </li>
            ))}
          </ul>
        )}
        <Pagination page={page} pageSize={RESULTS_PER_PAGE} total={total} onPage={setPage} />
      </div>
    </div>
  )
}

/** A removable active-filter pill, shown below the advanced panel. */
function FilterChip({ label, onRemove }: { label: string; onRemove: () => void }) {
  return (
    <span className="inline-flex items-center gap-1 h-6 pl-2 pr-1 rounded-full bg-primary-light text-primary text-xs">
      {label}
      <button type="button" onClick={onRemove} className="rounded-full hover:bg-primary/20 p-0.5" aria-label="×">
        <X size={12} />
      </button>
    </span>
  )
}

/** `ts_headline` wraps each match in `\u0001…\u0002` control-character
 *  markers (see `search_service.rs`) instead of HTML tags -- the backend never
 *  sends markup for post bodies (untrusted user content), so this renders
 *  highlights as plain React text nodes rather than through
 *  `dangerouslySetInnerHTML`. The markers always alternate start/end, so
 *  splitting on either one and alternating plain/highlighted reconstructs it. */
function renderSnippet(snippet: string) {
  const parts = snippet.split(/[\u0001\u0002]/)
  return parts.map((part, i) => (i % 2 === 1 ? <mark key={i} className="bg-primary-light text-primary rounded-sm px-0.5">{part}</mark> : part))
}

/** Author filter: a debounced user search that resolves to a chip once one
 *  is picked, mirroring the `@mention` picker's lookup but as a plain field
 *  rather than an inline textarea popup. */
function AuthorFilterField({
  authorId, authorLabel, onSelect, onClear, t,
}: {
  authorId: string | null
  authorLabel: string | null
  onSelect: (u: UserBrief) => void
  onClear: () => void
  t: TFunction
}) {
  const [q, setQ] = useState('')
  const [results, setResults] = useState<UserBrief[]>([])
  const [loading, setLoading] = useState(false)
  const [open, setOpen] = useState(false)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (q.trim().length < 2) { setResults([]); return }
    if (debounceRef.current) clearTimeout(debounceRef.current)
    setLoading(true)
    debounceRef.current = setTimeout(() => {
      forumApi.searchUsers(q.trim())
        .then(setResults)
        .catch(() => setResults([]))
        .finally(() => setLoading(false))
    }, 200)
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current) }
  }, [q])

  if (authorId) {
    return (
      <div className="flex items-center justify-between gap-2 h-9 px-3 rounded-lg border border-border bg-surface-1 text-sm">
        <span className="truncate text-text-primary">{authorLabel}</span>
        <button type="button" onClick={onClear} className="text-text-tertiary hover:text-text-primary shrink-0"
          aria-label={t('clear', { defaultValue: 'Effacer' })}>
          <X size={14} />
        </button>
      </div>
    )
  }

  return (
    <div className="relative">
      <Input
        value={q}
        onChange={(e) => { setQ(e.target.value); setOpen(true) }}
        onFocus={() => setOpen(true)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
        placeholder={t('search_by_author_ph', { defaultValue: "Nom d'utilisateur…" })}
      />
      {open && (loading || results.length > 0) && (
        <div className="absolute z-20 mt-1 w-full max-h-56 overflow-auto rounded-lg border border-border bg-surface-0 shadow-lg py-1">
          {loading && results.length === 0 ? (
            <div className="px-3 py-2 flex justify-center"><Spinner size="sm" /></div>
          ) : results.map(u => (
            <button
              key={u.id}
              type="button"
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => { onSelect(u); setQ(''); setOpen(false) }}
              className="w-full text-left px-3 py-1.5 text-sm hover:bg-surface-1 text-text-primary truncate"
            >
              {u.display_name || u.username}
              <span className="text-text-tertiary"> @{u.username}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

/** Forum multi-select for the advanced panel — a plain checkbox list, since
 *  forum counts on a self-hosted instance stay small enough that a filter
 *  field would be more friction than it saves. */
function ForumFilterList({
  selected, onToggle, t,
}: {
  selected: string[]
  onToggle: (id: string) => void
  t: TFunction
}) {
  const { data: forums, isLoading } = useQuery({
    queryKey: ['forum-all-forums-for-search'],
    queryFn: () => forumApi.listForums(),
  })

  if (isLoading) return <div className="py-2 flex justify-center"><Spinner size="sm" /></div>

  return (
    <div className="max-h-40 overflow-auto rounded-lg border border-border p-2 space-y-1">
      {(forums ?? []).map(f => (
        <Checkbox key={f.id} checked={selected.includes(f.id)} onChange={() => onToggle(f.id)} label={f.name} />
      ))}
      {(forums ?? []).length === 0 && (
        <div className="text-xs text-text-tertiary px-1 py-1">{t('no_forums', { defaultValue: 'Aucun forum' })}</div>
      )}
    </div>
  )
}
