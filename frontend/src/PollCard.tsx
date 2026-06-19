import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { BarChart3, Check } from 'lucide-react'
import { Button } from '@ui'
import { forumApi, type PollResults } from './api'

/** Renders a topic's poll: vote when not yet voted, results otherwise. */
export default function PollCard({ topicId }: { topicId: string }) {
  const { t } = useTranslation('forum')
  const [poll, setPoll] = useState<PollResults | null>(null)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [voting, setVoting] = useState(false)

  useEffect(() => { forumApi.getPoll(topicId).then(setPoll).catch(() => {}) }, [topicId])

  if (!poll) return null
  const showResults = poll.has_voted || poll.is_closed
  const total = poll.options.reduce((s, o) => s + o.votes, 0)

  const pick = (id: string) => {
    setSelected(prev => {
      const n = new Set(poll.poll.is_multiple ? prev : [])
      if (prev.has(id) && poll.poll.is_multiple) n.delete(id); else n.add(id)
      return n
    })
  }
  const submit = async () => {
    if (!selected.size) return
    setVoting(true)
    try { setPoll(await forumApi.votePoll(poll.poll.id, [...selected])) } finally { setVoting(false) }
  }

  return (
    <div className="rounded-xl border border-border bg-surface-0 p-4 mb-3">
      <div className="flex items-center gap-2 mb-3">
        <BarChart3 size={16} className="text-primary" />
        <h3 className="text-sm font-semibold text-text-primary">{poll.poll.question}</h3>
        {poll.is_closed && <span className="text-xs text-text-tertiary">· {t('poll_closed')}</span>}
      </div>
      <div className="space-y-2">
        {poll.options.map(o => {
          const pct = total > 0 ? Math.round((o.votes / total) * 100) : 0
          return showResults ? (
            <div key={o.id} className="relative rounded-lg border border-border overflow-hidden">
              <div className="absolute inset-0 bg-primary-light" style={{ width: `${pct}%` }} />
              <div className="relative flex items-center justify-between px-3 py-1.5 text-sm">
                <span className="flex items-center gap-1.5 text-text-primary">{o.me && <Check size={13} className="text-primary" />}{o.text}</span>
                <span className="text-text-secondary font-medium">{pct}% · {o.votes}</span>
              </div>
            </div>
          ) : (
            <button key={o.id} onClick={() => pick(o.id)}
              className={`w-full text-left px-3 py-1.5 rounded-lg border text-sm transition-colors
                          ${selected.has(o.id) ? 'border-primary bg-primary-light text-primary' : 'border-border hover:bg-surface-1 text-text-primary'}`}>
              {o.text}
            </button>
          )
        })}
      </div>
      <div className="flex items-center justify-between mt-3">
        <span className="text-xs text-text-tertiary">{t('poll_voters', { count: poll.total_voters })}</span>
        {!showResults && <Button variant="primary" size="sm" loading={voting} disabled={!selected.size} onClick={submit}>{t('poll_vote')}</Button>}
      </div>
    </div>
  )
}
