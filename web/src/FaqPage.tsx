import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ChevronDown, ChevronUp, HelpCircle } from 'lucide-react'
import { Spinner } from '@ui'
import { forumApi } from './api'
import PostBody from './PostBody'

/**
 * Public FAQ: a short, admin-curated list of question/answer
 * pairs (see `admin/ForumAdminPanel.tsx`), read here by every member as a
 * simple accordion — one question open at a time. Answers are Markdown,
 * rendered through the same sanitized `PostBody` renderer as post bodies.
 */
export default function FaqPage() {
  const { t } = useTranslation('forum')
  const { data: entries = [], isLoading } = useQuery({ queryKey: ['forum-faq'], queryFn: forumApi.listFaq })
  const [openId, setOpenId] = useState<string | null>(null)

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center gap-2 mb-4">
          <HelpCircle size={20} className="text-primary" />
          <h1 className="text-lg font-semibold text-text-primary">{t('faq', { defaultValue: 'FAQ' })}</h1>
        </div>

        {isLoading ? (
          <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
        ) : entries.length === 0 ? (
          <div className="rounded-xl border border-border bg-surface-0 px-4 py-16 text-center text-text-secondary">
            {t('no_faq_entries', { defaultValue: "Aucune question n'a encore été ajoutée." })}
          </div>
        ) : (
          <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border overflow-hidden">
            {entries.map(entry => {
              const open = openId === entry.id
              return (
                <li key={entry.id}>
                  <button
                    onClick={() => setOpenId(open ? null : entry.id)}
                    className="w-full flex items-center gap-2 px-4 py-3 text-left hover:bg-surface-1"
                  >
                    <span className="flex-1 text-sm font-medium text-text-primary">{entry.question}</span>
                    {open ? <ChevronUp size={16} className="text-text-tertiary shrink-0" /> : <ChevronDown size={16} className="text-text-tertiary shrink-0" />}
                  </button>
                  {open && (
                    <div className="px-4 pb-4">
                      <PostBody body={entry.answer_md} />
                    </div>
                  )}
                </li>
              )
            })}
          </ul>
        )}
      </div>
    </div>
  )
}
