import { useTranslation } from 'react-i18next'
import { Plus, X } from 'lucide-react'
import { Input } from '@ui'
import type { NewPoll } from './api'

const MAX_OPTIONS = 20

/** Duration presets (in days); 0 means the poll never closes. */
const DURATIONS = [0, 1, 3, 7, 14, 30]

/**
 * Controlled editor for an optional poll attached to a new topic. Enforces the
 * same shape the server validates (2–20 options); the parent decides whether a
 * poll is attached at all.
 */
export default function PollComposer({ value, onChange }: { value: NewPoll; onChange: (p: NewPoll) => void }) {
  const { t } = useTranslation('forum')

  const setOption = (i: number, text: string) =>
    onChange({ ...value, options: value.options.map((o, idx) => (idx === i ? text : o)) })
  const addOption = () => {
    if (value.options.length < MAX_OPTIONS) onChange({ ...value, options: [...value.options, ''] })
  }
  const removeOption = (i: number) => {
    if (value.options.length > 2) onChange({ ...value, options: value.options.filter((_, idx) => idx !== i) })
  }
  const setDuration = (days: number) => {
    const closes_at = days > 0 ? new Date(Date.now() + days * 86_400_000).toISOString() : null
    onChange({ ...value, closes_at })
  }

  // Which preset (if any) the current closes_at roughly matches, for highlighting.
  const activeDays = value.closes_at
    ? DURATIONS.find(d => d > 0 && Math.abs((new Date(value.closes_at!).getTime() - Date.now()) / 86_400_000 - d) < 0.5) ?? -1
    : 0

  return (
    <div className="rounded-lg border border-border bg-surface-1 p-3 flex flex-col gap-2">
      <Input
        label={t('poll_question', { defaultValue: 'Poll question' })}
        value={value.question}
        onChange={(e) => onChange({ ...value, question: e.target.value })}
        placeholder={t('poll_question', { defaultValue: 'Poll question' })}
      />

      <div className="flex flex-col gap-1.5">
        {value.options.map((opt, i) => (
          <div key={i} className="flex items-center gap-1.5">
            <input
              value={opt}
              onChange={(e) => setOption(i, e.target.value)}
              placeholder={t('poll_option', { defaultValue: 'Option {{n}}', n: i + 1 })}
              maxLength={200}
              className="flex-1 h-8 px-2 rounded-md border border-border bg-surface-0 text-sm text-text-primary outline-none focus:border-primary"
            />
            <button
              type="button"
              onClick={() => removeOption(i)}
              disabled={value.options.length <= 2}
              title={t('remove', { defaultValue: 'Remove' })}
              className="p-1.5 rounded text-text-tertiary hover:bg-surface-2 disabled:opacity-30 disabled:pointer-events-none"
            >
              <X size={14} />
            </button>
          </div>
        ))}
        {value.options.length < MAX_OPTIONS && (
          <button type="button" onClick={addOption} className="self-start inline-flex items-center gap-1 text-xs text-primary hover:underline mt-0.5">
            <Plus size={13} /> {t('add_option', { defaultValue: 'Add option' })}
          </button>
        )}
      </div>

      <label className="inline-flex items-center gap-2 text-xs text-text-secondary">
        <input type="checkbox" checked={value.is_multiple} onChange={(e) => onChange({ ...value, is_multiple: e.target.checked })} />
        {t('poll_multiple', { defaultValue: 'Allow selecting several options' })}
      </label>

      <div>
        <div className="text-xs font-medium text-text-secondary mb-1">{t('poll_duration', { defaultValue: 'Runs for' })}</div>
        <div className="flex gap-1 flex-wrap">
          {DURATIONS.map(d => (
            <button
              key={d}
              type="button"
              onClick={() => setDuration(d)}
              className={`px-2 py-1 rounded-md text-xs border ${activeDays === d ? 'bg-primary text-white border-primary' : 'border-border text-text-secondary hover:bg-surface-0'}`}
            >
              {d === 0 ? t('poll_no_end', { defaultValue: 'No end' }) : t('poll_days', { defaultValue: '{{count}} days', count: d })}
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
