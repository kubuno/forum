// Word censor section (Modules ▸ Forum ▸ Moderation), instance-wide, applied
// server-side.
//
// Every word/phrase here is substituted server-side, at render time, in every
// post body (`CensorService`, see `handlers/posts.rs::list`) — the admin's
// list is never sent to the client to filter locally, so there is nothing
// for a member to bypass by inspecting the response.

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, Ban, Trash2, Check, X } from 'lucide-react'
import { Button, Input, Spinner, ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi } from '../../api'

export function CensoredWordsSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const { data: words = [], isLoading } = useQuery({ queryKey: ['forum-censored-words'], queryFn: forumApi.listCensoredWords })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['forum-censored-words'] })
  const [addingWord, setAddingWord] = useState(false)
  const [newPattern, setNewPattern] = useState('')
  const [newReplacement, setNewReplacement] = useState('')

  // Same in-place add row as the report reasons above, instead of a chain of
  // `prompt()` calls: one row carries both the word and its replacement.
  const startAddWord = () => { setAddingWord(true); setNewPattern(''); setNewReplacement('') }
  const cancelAddWord = () => setAddingWord(false)
  const submitAddWord = async () => {
    if (!newPattern.trim()) return
    await forumApi.createCensoredWord({ pattern: newPattern.trim(), replacement: newReplacement.trim() || null })
    cancelAddWord(); invalidate()
  }
  const removeWord = async (id: string, pattern: string) => {
    if (await confirm({ title: t('delete'), message: pattern, confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteCensoredWord(id); invalidate()
    }
  }

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-sm font-semibold text-text-secondary">{t('censored_words', { defaultValue: 'Censure de mots' })}</h2>
        {!addingWord && (
          <Button variant="secondary" size="sm" icon={<Plus size={15} />} onClick={startAddWord}>
            {t('new_censored_word', { defaultValue: 'Nouveau mot' })}
          </Button>
        )}
      </div>
      <p className="text-xs text-text-tertiary mb-2">
        {t('censored_words_desc', {
          defaultValue: "Chaque mot est remplacé automatiquement dans les messages, côté serveur — insensible à la casse, sur un mot entier.",
        })}
      </p>
      <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
        {addingWord && (
          <li className="flex items-center gap-2 px-3 py-2">
            <Input
              autoFocus
              value={newPattern}
              onChange={(e) => setNewPattern(e.target.value)}
              placeholder={t('censored_word_pattern', { defaultValue: 'Mot à censurer' })}
              className="flex-1"
              onKeyDown={(e) => { if (e.key === 'Enter') submitAddWord(); if (e.key === 'Escape') cancelAddWord() }}
            />
            <Input
              value={newReplacement}
              onChange={(e) => setNewReplacement(e.target.value)}
              placeholder="***"
              className="w-28"
              onKeyDown={(e) => { if (e.key === 'Enter') submitAddWord(); if (e.key === 'Escape') cancelAddWord() }}
            />
            <button onClick={cancelAddWord} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('cancel')}><X size={15} /></button>
            <button onClick={submitAddWord} disabled={!newPattern.trim()} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary disabled:opacity-30 disabled:pointer-events-none" title={t('create')}><Check size={15} /></button>
          </li>
        )}
        {words.map(w => (
          <li key={w.id} className="flex items-center gap-2 px-3 py-2">
            <Ban size={15} className="text-primary shrink-0" />
            <span className="flex-1 text-sm text-text-primary truncate">{w.pattern}</span>
            <span className="text-xs text-text-tertiary font-mono">{w.replacement}</span>
            <button onClick={() => removeWord(w.id, w.pattern)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
          </li>
        ))}
        {words.length === 0 && !addingWord && <li className="px-3 py-3 text-sm text-text-tertiary">{t('no_censored_words', { defaultValue: 'Aucun mot censuré.' })}</li>}
      </ul>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
