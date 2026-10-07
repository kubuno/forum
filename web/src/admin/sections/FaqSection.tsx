// FAQ section (Modules ▸ Forum ▸ Structure): an admin-curated list of
// question/answer pairs, read by every member.
//
// Every entry here is shown to members on the dedicated `/forum/faq` page
// (`FaqPage.tsx`), rendered through the same sanitized Markdown renderer as
// post bodies. Editing happens IN PLACE, like the other sections on this
// page: adding opens an inline form row, editing swaps a row's display for
// the same form pre-filled — never a modal.

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, HelpCircle, Pencil, Trash2, Check, X } from 'lucide-react'
import { Button, Input, Spinner, ConfirmDialog, Textarea } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi, type FaqEntry } from '../../api'

interface FaqFormValues {
  question: string
  answer_md: string
  position: number
}

function FaqEntryForm({ initial, onCancel, onSubmit, busy }: {
  initial?: FaqEntry
  onCancel: () => void
  onSubmit: (values: FaqFormValues) => void
  busy: boolean
}) {
  const { t } = useTranslation('forum')
  const [question, setQuestion] = useState(initial?.question ?? '')
  const [answerMd, setAnswerMd] = useState(initial?.answer_md ?? '')
  const [position, setPosition] = useState(String(initial?.position ?? 0))

  const valid = question.trim().length > 0 && answerMd.trim().length > 0

  const submit = () => {
    if (!valid || busy) return
    onSubmit({ question: question.trim(), answer_md: answerMd.trim(), position: Number(position) || 0 })
  }

  return (
    <li className="flex flex-col gap-2 px-3 py-3">
      <Input
        autoFocus
        value={question}
        onChange={(e) => setQuestion(e.target.value)}
        placeholder={t('faq_question', { defaultValue: 'Question' })}
      />
      <Textarea
        value={answerMd}
        onChange={(e) => setAnswerMd(e.target.value)}
        rows={4}
        placeholder={t('faq_answer_ph', { defaultValue: 'Réponse (Markdown supporté)…' })}
      />
      <Input
        type="number"
        value={position}
        onChange={(e) => setPosition(e.target.value)}
        className="w-24"
        label={t('position', { defaultValue: 'Position' })}
      />
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" icon={<X size={14} />} onClick={onCancel}>{t('cancel')}</Button>
        <Button variant="primary" size="sm" icon={<Check size={14} />} disabled={!valid} loading={busy} onClick={submit}>
          {initial ? t('save') : t('create')}
        </Button>
      </div>
    </li>
  )
}

export function FaqSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const { data: entries = [], isLoading } = useQuery({ queryKey: ['forum-faq'], queryFn: forumApi.listFaq })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['forum-faq'] })
  const [adding, setAdding] = useState(false)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const create = async (values: FaqFormValues) => {
    setBusy(true)
    try {
      await forumApi.createFaq(values)
      setAdding(false)
      invalidate()
    } finally {
      setBusy(false)
    }
  }
  const update = async (id: string, values: FaqFormValues) => {
    setBusy(true)
    try {
      await forumApi.updateFaq(id, values)
      setEditingId(null)
      invalidate()
    } finally {
      setBusy(false)
    }
  }
  const remove = async (entry: FaqEntry) => {
    if (await confirm({ title: t('delete'), message: entry.question, confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteFaq(entry.id)
      invalidate()
    }
  }

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-sm font-semibold text-text-secondary">{t('faq', { defaultValue: 'FAQ' })}</h2>
        {!adding && (
          <Button variant="secondary" size="sm" icon={<Plus size={15} />} onClick={() => setAdding(true)}>
            {t('new_faq_entry', { defaultValue: 'Nouvelle question' })}
          </Button>
        )}
      </div>
      <p className="text-xs text-text-tertiary mb-2">
        {t('faq_desc', {
          defaultValue: 'Affichée à tous les membres sur une page dédiée. La réponse accepte le Markdown.',
        })}
      </p>
      <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
        {adding && <FaqEntryForm onCancel={() => setAdding(false)} onSubmit={create} busy={busy} />}
        {entries.map(entry => (
          editingId === entry.id ? (
            <FaqEntryForm key={entry.id} initial={entry} onCancel={() => setEditingId(null)} onSubmit={(values) => update(entry.id, values)} busy={busy} />
          ) : (
            <li key={entry.id} className="flex items-center gap-2 px-3 py-2">
              <HelpCircle size={15} className="text-primary shrink-0" />
              <span className="flex-1 min-w-0 text-sm text-text-primary truncate">{entry.question}</span>
              <button onClick={() => setEditingId(entry.id)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('edit')}><Pencil size={15} /></button>
              <button onClick={() => remove(entry)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
            </li>
          )
        ))}
        {entries.length === 0 && !adding && <li className="px-3 py-3 text-sm text-text-tertiary">{t('no_faq_entries', { defaultValue: "Aucune question n'a encore été ajoutée." })}</li>}
      </ul>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
