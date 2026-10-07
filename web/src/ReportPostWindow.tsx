import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Flag } from 'lucide-react'
import { FloatingWindow, Button, Textarea, Spinner } from '@ui'
import { forumApi } from './api'

interface Props {
  postId: string
  onClose: () => void
  onSent: () => void
}

/** Reports a post: a predefined reason (chip) picked from the admin-curated
 *  list, plus an optional free-text comment. Replaces the old single
 *  `prompt()` — no browser dialog, and the reporter now points at a category
 *  a moderator can triage on sight instead of parsing free text every time. */
export default function ReportPostWindow({ postId, onClose, onSent }: Props) {
  const { t } = useTranslation('forum')
  const { data: reasons = [], isLoading } = useQuery({ queryKey: ['forum-report-reasons'], queryFn: forumApi.listReportReasons })
  const [reasonId, setReasonId] = useState<string | null>(null)
  const [comment, setComment] = useState('')
  const [busy, setBusy] = useState(false)

  const selected = reasons.find(r => r.id === reasonId)
  // A submission needs SOMETHING to tell the moderators — a picked chip, or,
  // failing that, a comment. Never both empty.
  const canSubmit = !!reasonId || comment.trim().length > 0

  const submit = async () => {
    if (!canSubmit || busy) return
    setBusy(true)
    try {
      // `reason` (free text) stays mandatory server-side, so a chip pick
      // without a comment falls back to the chip's own title.
      const reason = comment.trim() || selected?.title || t('report_post')
      await forumApi.reportPost(postId, { reason, reason_id: reasonId })
      onSent()
    } finally {
      setBusy(false)
    }
  }

  return (
    <FloatingWindow title={t('report_post')} icon={<Flag size={18} />} onClose={onClose} defaultWidth={440} defaultHeight={420}>
      <div className="p-4 flex flex-col gap-3 h-full">
        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1.5">
            {t('report_choose_reason', { defaultValue: 'Choisissez une raison' })}
          </label>
          {isLoading ? (
            <div className="py-4 flex justify-center"><Spinner size="sm" /></div>
          ) : (
            <div className="flex flex-wrap gap-1.5">
              {reasons.map(r => (
                <button
                  key={r.id}
                  type="button"
                  title={r.description ?? undefined}
                  onClick={() => setReasonId(prev => (prev === r.id ? null : r.id))}
                  className={`px-2.5 py-1 rounded-full text-xs font-medium border transition-colors ${
                    reasonId === r.id
                      ? 'bg-primary text-white border-primary'
                      : 'bg-surface-1 text-text-secondary border-border hover:bg-surface-2'
                  }`}
                >
                  {r.title}
                </button>
              ))}
              {reasons.length === 0 && (
                <span className="text-xs text-text-tertiary">{t('no_report_reasons', { defaultValue: 'Aucune raison prédéfinie.' })}</span>
              )}
            </div>
          )}
        </div>
        <Textarea
          label={t('report_comment', { defaultValue: 'Commentaire (optionnel)' })}
          value={comment}
          onChange={(e) => setComment(e.target.value)}
          rows={4}
          placeholder={t('report_reason')}
        />
        <div className="flex justify-end gap-2 mt-auto">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} disabled={!canSubmit} onClick={submit}>{t('report')}</Button>
        </div>
      </div>
    </FloatingWindow>
  )
}
