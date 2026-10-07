// Ranks section (Modules ▸ Forum ▸ Ranks): the instance-wide ladder of ranks
// based on post count, plus manual assignment of a special rank to a member.

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, Award, Pencil, Trash2, UserPlus, Check, X } from 'lucide-react'
import { Button, Input, Dropdown, Spinner, ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi, type Rank, type UserBrief } from '../../api'

// ── Assignation manuelle d'un rang spécial ───────────────────────────────────

function AssignRankForm() {
  const { t } = useTranslation('forum')
  const { data: ranks = [] } = useQuery({ queryKey: ['forum-ranks'], queryFn: forumApi.listRanks })
  const specialRanks = ranks.filter(r => r.is_special)
  const [q, setQ] = useState('')
  const [found, setFound] = useState<UserBrief[]>([])
  const [selectedUser, setSelectedUser] = useState<UserBrief | null>(null)
  const [rankId, setRankId] = useState('')
  const [busy, setBusy] = useState(false)

  const search = async (value: string) => {
    setQ(value)
    setSelectedUser(null)
    if (value.trim().length < 2) { setFound([]); return }
    setFound(await forumApi.searchUsers(value.trim()))
  }

  const assign = async () => {
    if (!selectedUser || !rankId || busy) return
    setBusy(true)
    try {
      await forumApi.assignRank(selectedUser.id, rankId)
      setSelectedUser(null); setQ(''); setFound([]); setRankId('')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="rounded-xl border border-border bg-surface-0 p-3 flex flex-col gap-2 mb-3">
      <h3 className="text-xs font-semibold text-text-secondary">{t('assign_special_rank', { defaultValue: 'Assign a special rank' })}</h3>
      <div className="relative">
        <Input
          value={selectedUser ? (selectedUser.display_name || selectedUser.username) : q}
          onChange={(e) => search(e.target.value)}
          leftIcon={<UserPlus size={16} />}
          placeholder={t('search_member', { defaultValue: 'Search a member…' })}
        />
        {found.length > 0 && !selectedUser && (
          <ul className="absolute z-10 left-0 right-0 mt-1 bg-surface-0 border border-border rounded-lg shadow-lg max-h-48 overflow-auto">
            {found.map(u => (
              <li key={u.id}>
                <button onClick={() => { setSelectedUser(u); setFound([]) }} className="w-full text-left px-3 py-2 text-sm hover:bg-surface-1">
                  {u.display_name || u.username}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="flex items-end gap-2">
        <div className="flex-1">
          <Dropdown
            options={specialRanks.map(r => ({ value: r.id, label: r.title }))}
            value={rankId}
            onChange={setRankId}
            placeholder={t('special_rank')}
            width="100%"
            height={36}
          />
        </div>
        <Button variant="secondary" disabled={!selectedUser || !rankId} loading={busy} onClick={assign}>
          {t('assign_rank_action', { defaultValue: 'Assign' })}
        </Button>
      </div>
    </div>
  )
}

// ── Rangs (instance-wide) ───────────────────────────────────────────────────────

export function RanksSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const { data: ranks = [], isLoading } = useQuery({ queryKey: ['forum-ranks'], queryFn: forumApi.listRanks })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['forum-ranks'] })
  const [addingRank, setAddingRank] = useState(false)
  const [newRankTitle, setNewRankTitle] = useState('')
  const [newRankMinPosts, setNewRankMinPosts] = useState('0')
  const [editingRankId, setEditingRankId] = useState<string | null>(null)
  const [editRankMinPosts, setEditRankMinPosts] = useState('')

  // Replaces the old chain of two `prompt()` calls (title, then min posts)
  // with a single inline row carrying both fields at once.
  const startAddRank = () => { setAddingRank(true); setNewRankTitle(''); setNewRankMinPosts('0') }
  const cancelAddRank = () => setAddingRank(false)
  const submitAddRank = async () => {
    if (!newRankTitle.trim()) return
    await forumApi.createRank({ title: newRankTitle.trim(), min_posts: Number(newRankMinPosts) || 0 })
    cancelAddRank(); invalidate()
  }
  const startEditRank = (r: Rank) => { setEditingRankId(r.id); setEditRankMinPosts(String(r.min_posts)) }
  const cancelEditRank = () => setEditingRankId(null)
  const submitEditRank = async () => {
    if (!editingRankId) return
    await forumApi.updateRank(editingRankId, { min_posts: Number(editRankMinPosts) || 0 })
    cancelEditRank(); invalidate()
  }
  const removeRank = async (r: Rank) => {
    if (await confirm({ title: t('delete'), message: r.title, confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteRank(r.id); invalidate()
    }
  }

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  return (
    <div>
      <AssignRankForm />
      <div className="flex justify-end mb-2">
        {!addingRank && <Button variant="secondary" size="sm" icon={<Plus size={15} />} onClick={startAddRank}>{t('new_rank')}</Button>}
      </div>
      <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
        {addingRank && (
          <li className="flex items-center gap-2 px-3 py-2">
            <Award size={15} className="text-primary shrink-0" />
            <Input autoFocus value={newRankTitle} onChange={(e) => setNewRankTitle(e.target.value)} placeholder={t('title')} className="flex-1" />
            <Input
              type="number"
              value={newRankMinPosts}
              onChange={(e) => setNewRankMinPosts(e.target.value)}
              placeholder="0"
              className="w-24"
              title={t('min_posts')}
            />
            <button onClick={cancelAddRank} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('cancel')}><X size={15} /></button>
            <button onClick={submitAddRank} disabled={!newRankTitle.trim()} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary disabled:opacity-30 disabled:pointer-events-none" title={t('create')}><Check size={15} /></button>
          </li>
        )}
        {ranks.map(r => (
          <li key={r.id} className="flex items-center gap-2 px-3 py-2">
            <Award size={15} className="text-primary shrink-0" />
            <span className="flex-1 text-sm text-text-primary truncate">{r.title}</span>
            {editingRankId === r.id ? (
              <>
                <Input
                  autoFocus
                  type="number"
                  value={editRankMinPosts}
                  onChange={(e) => setEditRankMinPosts(e.target.value)}
                  className="w-24"
                  onKeyDown={(e) => { if (e.key === 'Enter') submitEditRank(); if (e.key === 'Escape') cancelEditRank() }}
                />
                <button onClick={cancelEditRank} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('cancel')}><X size={15} /></button>
                <button onClick={submitEditRank} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('save')}><Check size={15} /></button>
              </>
            ) : (
              <>
                <span className="text-xs text-text-tertiary">{t('member_posts', { count: r.min_posts })}</span>
                {!r.is_special && <button onClick={() => startEditRank(r)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('edit')}><Pencil size={15} /></button>}
                <button onClick={() => removeRank(r)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
              </>
            )}
          </li>
        ))}
      </ul>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
