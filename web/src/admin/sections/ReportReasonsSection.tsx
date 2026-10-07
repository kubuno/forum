// Report reasons section (Modules ▸ Forum ▸ Moderation): a moderator-curated
// list of chips a member picks from when reporting a post, instance-wide.

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, Flag, Trash2, Check, X } from 'lucide-react'
import { Button, Input, Spinner, ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi } from '../../api'

export function ReportReasonsSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const { data: reasons = [], isLoading } = useQuery({ queryKey: ['forum-report-reasons'], queryFn: forumApi.listReportReasons })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['forum-report-reasons'] })
  const [addingReason, setAddingReason] = useState(false)
  const [newReasonTitle, setNewReasonTitle] = useState('')
  const [newReasonDescription, setNewReasonDescription] = useState('')

  // Replaces the old chain of two `prompt()` calls (title, then optional
  // description) with a single inline row carrying both fields at once.
  const startAddReason = () => { setAddingReason(true); setNewReasonTitle(''); setNewReasonDescription('') }
  const cancelAddReason = () => setAddingReason(false)
  const submitAddReason = async () => {
    if (!newReasonTitle.trim()) return
    await forumApi.createReportReason({ title: newReasonTitle.trim(), description: newReasonDescription.trim() || null, position: reasons.length })
    cancelAddReason(); invalidate()
  }
  const removeReason = async (id: string, title: string) => {
    if (await confirm({ title: t('delete'), message: title, confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteReportReason(id); invalidate()
    }
  }

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-sm font-semibold text-text-secondary">{t('report_reasons', { defaultValue: 'Raisons de signalement' })}</h2>
        {!addingReason && (
          <Button variant="secondary" size="sm" icon={<Plus size={15} />} onClick={startAddReason}>
            {t('new_report_reason', { defaultValue: 'Nouvelle raison' })}
          </Button>
        )}
      </div>
      <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
        {addingReason && (
          <li className="flex flex-col gap-2 px-3 py-2">
            <Input autoFocus value={newReasonTitle} onChange={(e) => setNewReasonTitle(e.target.value)} placeholder={t('title')} />
            <Input
              value={newReasonDescription}
              onChange={(e) => setNewReasonDescription(e.target.value)}
              placeholder={t('report_reason_description', { defaultValue: 'Description (optionnelle)' })}
            />
            <div className="flex justify-end gap-2">
              <Button variant="ghost" size="sm" icon={<X size={14} />} onClick={cancelAddReason}>{t('cancel')}</Button>
              <Button variant="primary" size="sm" icon={<Check size={14} />} disabled={!newReasonTitle.trim()} onClick={submitAddReason}>{t('create')}</Button>
            </div>
          </li>
        )}
        {reasons.map(r => (
          <li key={r.id} className="flex items-center gap-2 px-3 py-2">
            <Flag size={15} className="text-primary shrink-0" />
            <div className="flex-1 min-w-0">
              <div className="text-sm text-text-primary truncate">{r.title}</div>
              {r.description && <div className="text-xs text-text-tertiary truncate">{r.description}</div>}
            </div>
            <button onClick={() => removeReason(r.id, r.title)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
          </li>
        ))}
        {reasons.length === 0 && <li className="px-3 py-3 text-sm text-text-tertiary">{t('no_report_reasons', { defaultValue: 'Aucune raison prédéfinie.' })}</li>}
      </ul>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
