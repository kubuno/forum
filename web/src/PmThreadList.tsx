import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { MessageSquarePlus, Inbox } from 'lucide-react'
import { Spinner, Button } from '@ui'
import { useAuthStore } from '@kubuno/sdk'
import { forumApi, type PmThreadSummary } from './api'
import { useResolveUsers, useUser, userLabel } from './users'
import { AuthorAvatar } from './Author'
import { timeAgo } from './helpers'
import PmComposeWindow from './PmComposeWindow'

/** Private-message inbox: every conversation the current user is part of,
 *  newest first (the backend already orders by last activity). */
export default function PmThreadList() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const me = useAuthStore(s => s.user)
  const [composing, setComposing] = useState(false)

  const { data: threads = [], isLoading } = useQuery({
    queryKey: ['forum-pm-threads'],
    queryFn: forumApi.listPmThreads,
  })

  const allParticipantIds = [...new Set(threads.flatMap(th => th.participant_ids))]
  useResolveUsers(allParticipantIds)

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-lg font-semibold text-text-primary">{t('private_messages', { defaultValue: 'Messages privés' })}</h1>
          <Button variant="primary" size="sm" icon={<MessageSquarePlus size={15} />} onClick={() => setComposing(true)}>
            {t('new_message', { defaultValue: 'Nouveau message' })}
          </Button>
        </div>

        <div className="rounded-xl border border-border overflow-hidden bg-surface-0">
          {isLoading ? (
            <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
          ) : threads.length === 0 ? (
            <div className="px-4 py-16 flex flex-col items-center gap-2 text-center text-text-secondary">
              <Inbox size={28} className="text-text-tertiary" />
              {t('no_pm_threads', { defaultValue: 'Aucune conversation pour l’instant.' })}
            </div>
          ) : (
            <ul className="divide-y divide-border">
              {threads.map(th => (
                <PmThreadRow key={th.id} thread={th} meId={me?.id} onOpen={() => navigate(`/forum/pm/${th.id}`)} />
              ))}
            </ul>
          )}
        </div>
      </div>

      {composing && <PmComposeWindow onClose={() => setComposing(false)} />}
    </div>
  )
}

function PmThreadRow({ thread, meId, onOpen }: {
  thread: PmThreadSummary
  meId: string | undefined
  onOpen: () => void
}) {
  const { t } = useTranslation('forum')
  const others = thread.participant_ids.filter(id => id !== meId)
  const primaryOther = others[0]
  const other = useUser(primaryOther)
  const unread = thread.unread_count > 0

  const title = thread.subject?.trim()
    || (others.length > 1
      ? t('pm_group_title', { defaultValue: '{{name}} et {{count}} autres', name: userLabel(other), count: others.length - 1 })
      : userLabel(other))

  return (
    <li>
      <button onClick={onOpen} className="w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-surface-1">
        <AuthorAvatar id={primaryOther ?? null} size={36} />
        <div className="min-w-0 flex-1">
          <div className={`truncate ${unread ? 'font-semibold text-text-primary' : 'font-medium text-text-primary'}`}>{title}</div>
          <div className={`text-sm truncate ${unread ? 'text-text-primary' : 'text-text-tertiary'}`}>{thread.last_message_preview}</div>
        </div>
        <div className="flex flex-col items-end gap-1 shrink-0">
          <span className="text-xs text-text-tertiary">{timeAgo(thread.last_message_at)}</span>
          {unread && (
            <span className="text-xs bg-primary text-white rounded-full min-w-[18px] h-[18px] px-1 flex items-center justify-center">
              {thread.unread_count > 99 ? '99+' : thread.unread_count}
            </span>
          )}
        </div>
      </button>
    </li>
  )
}
