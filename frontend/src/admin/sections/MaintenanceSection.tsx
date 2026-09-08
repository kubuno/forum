// Maintenance section (Modules ▸ Forum ▸ Structure): bulk purge of inactive
// topics, per forum.
//
// Soft-deletes every ordinary, non-pinned, non-solved, poll-less topic of a
// forum whose last activity is older than N days. The preview (`dry_run: true`)
// never writes anything — it only counts — so the admin sees the impact before
// committing, and the actual purge still runs through `useConfirm` (never the
// browser's `confirm()`).

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Eraser } from 'lucide-react'
import { Button, Input, Dropdown, ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi } from '../../api'

export function MaintenanceSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const { data: forums = [] } = useQuery({ queryKey: ['forum-all-forums'], queryFn: () => forumApi.listForums() })
  const [forumId, setForumId] = useState('')
  const [days, setDays] = useState('90')
  const [preview, setPreview] = useState<number | null>(null)
  const [busy, setBusy] = useState(false)

  const daysNum = Number(days)
  const valid = !!forumId && Number.isInteger(daysNum) && daysNum >= 1

  const pickForum = (id: string) => { setForumId(id); setPreview(null) }
  const editDays = (value: string) => { setDays(value); setPreview(null) }

  const runPreview = async () => {
    if (!valid || busy) return
    setBusy(true)
    try {
      setPreview(await forumApi.pruneForum(forumId, { days: daysNum, dry_run: true }))
    } finally {
      setBusy(false)
    }
  }

  const runPrune = async () => {
    if (!valid || busy) return
    setBusy(true)
    let count: number
    try {
      count = await forumApi.pruneForum(forumId, { days: daysNum, dry_run: true })
      setPreview(count)
    } finally {
      setBusy(false)
    }
    if (count === 0) return
    const ok = await confirm({
      title: t('prune_forum_title', { defaultValue: 'Purger les sujets inactifs' }),
      message: t('prune_forum_confirm', { defaultValue: '{{count}} sujet(s) seront déplacés dans la corbeille.', count }),
      confirmLabel: t('prune_forum_action', { defaultValue: 'Purger' }),
      variant: 'danger',
    })
    if (!ok) return
    setBusy(true)
    try {
      await forumApi.pruneForum(forumId, { days: daysNum, dry_run: false })
      setPreview(0)
      qc.invalidateQueries({ queryKey: ['forum-all-forums'] })
      qc.invalidateQueries({ queryKey: ['forum-forums'] })
      qc.invalidateQueries({ queryKey: ['forum-topics'] })
    } finally {
      setBusy(false)
    }
  }

  return (
    <section>
      <h2 className="text-sm font-semibold text-text-secondary mb-2">{t('maintenance', { defaultValue: 'Maintenance' })}</h2>
      <div className="rounded-xl border border-border bg-surface-0 p-3 flex flex-col gap-2">
        <p className="text-xs text-text-tertiary">
          {t('prune_forum_desc', {
            defaultValue: "Déplace vers la corbeille les sujets d'un forum sans activité depuis N jours. Les sujets épinglés, résolus ou avec un sondage sont ignorés.",
          })}
        </p>
        <div className="flex items-end gap-2 flex-wrap">
          <div className="w-56">
            <Dropdown
              options={forums.map(f => ({ value: f.id, label: f.name }))}
              value={forumId}
              onChange={pickForum}
              placeholder={t('forum', { defaultValue: 'Forum' })}
              width="100%"
              height={36}
            />
          </div>
          <Input
            label={t('prune_forum_days', { defaultValue: "Jours d'inactivité" })}
            type="number"
            min={1}
            value={days}
            onChange={(e) => editDays(e.target.value)}
            className="w-40"
          />
          <Button variant="secondary" icon={<Eraser size={15} />} disabled={!valid} loading={busy} onClick={runPreview}>
            {t('preview', { defaultValue: 'Aperçu' })}
          </Button>
          <Button variant="danger" disabled={!valid} loading={busy} onClick={runPrune}>
            {t('prune_forum_action', { defaultValue: 'Purger' })}
          </Button>
        </div>
        {preview !== null && (
          <p className="text-xs text-text-secondary">
            {preview === 0
              ? t('prune_forum_none', { defaultValue: 'Aucun sujet à purger.' })
              : t('prune_forum_preview', { defaultValue: '{{count}} sujet(s) seront purgés.', count: preview })}
          </p>
        )}
      </div>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </section>
  )
}
