import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  FolderPlus, MessagesSquare, Pencil, Trash2, Lock, Unlock, Shield, KeyRound, Plus, Award,
} from 'lucide-react'
import { Tabs, Button, Input, Dropdown, Spinner, ConfirmDialog, type TabDef } from '@ui'
import { prompt, useConfirm } from '@kubuno/sdk'
import { forumApi, type Category, type Forum, type Rank } from './api'
import PostEditor from './PostEditor'
import ModeratorsWindow from './ModeratorsWindow'
import PermissionsWindow from './PermissionsWindow'

type Tab = 'structure' | 'ranks' | 'profile'

export default function ForumSettingsPage() {
  const { t } = useTranslation('forum')
  const [tab, setTab] = useState<Tab>('structure')
  const tabs: TabDef<Tab>[] = [
    { id: 'structure', label: t('manage_structure'), icon: MessagesSquare },
    { id: 'ranks', label: t('ranks'), icon: Award },
    { id: 'profile', label: t('profile'), icon: Pencil },
  ]
  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <h1 className="text-lg font-semibold text-text-primary mb-3">{t('settings')}</h1>
        <Tabs tabs={tabs} value={tab} onChange={setTab} className="mb-4" />
        {tab === 'structure' && <StructureTab />}
        {tab === 'ranks' && <RanksTab />}
        {tab === 'profile' && <ProfileTab />}
      </div>
    </div>
  )
}

function StructureTab() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const [modForum, setModForum] = useState<string | null>(null)
  const [permForum, setPermForum] = useState<string | null>(null)
  const [newForumCat, setNewForumCat] = useState('')
  const [newForumName, setNewForumName] = useState('')

  const { data: categories = [], isLoading: lc } = useQuery({ queryKey: ['forum-categories'], queryFn: forumApi.listCategories })
  const { data: forums = [], isLoading: lf } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ['forum-categories'] })
    qc.invalidateQueries({ queryKey: ['forum-forums'] })
  }

  const addCategory = async () => {
    const name = await prompt({ title: t('new_category'), placeholder: t('name'), confirmLabel: t('create') })
    if (name?.trim()) { await forumApi.createCategory({ name: name.trim() }); invalidate() }
  }
  const renameCategory = async (c: Category) => {
    const name = await prompt({ title: t('edit'), placeholder: t('name'), defaultValue: c.name, confirmLabel: t('save') })
    if (name?.trim()) { await forumApi.updateCategory(c.id, { name: name.trim() }); invalidate() }
  }
  const removeCategory = async (c: Category) => {
    if (await confirm({ title: t('delete_category'), message: t('confirm_delete_category'), confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteCategory(c.id); invalidate()
    }
  }
  const addForum = async () => {
    if (!newForumCat || !newForumName.trim()) return
    await forumApi.createForum({ category_id: newForumCat, name: newForumName.trim() })
    setNewForumName(''); invalidate()
  }
  const renameForum = async (f: Forum) => {
    const name = await prompt({ title: t('edit'), placeholder: t('name'), defaultValue: f.name, confirmLabel: t('save') })
    if (name?.trim()) { await forumApi.updateForum(f.id, { name: name.trim() }); invalidate() }
  }
  const removeForum = async (f: Forum) => {
    if (await confirm({ title: t('delete_forum'), message: t('confirm_delete_forum'), confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteForum(f.id); invalidate()
    }
  }
  const toggleLock = async (f: Forum) => { await forumApi.updateForum(f.id, { is_locked: !f.is_locked }); invalidate() }

  if (lc || lf) return <div className="py-10 flex justify-center"><Spinner /></div>

  return (
    <div className="space-y-5">
      {/* Categories */}
      <section>
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-sm font-semibold text-text-secondary">{t('categories')}</h2>
          <Button variant="secondary" size="sm" icon={<FolderPlus size={15} />} onClick={addCategory}>{t('new_category')}</Button>
        </div>
        <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
          {categories.map(c => (
            <li key={c.id} className="flex items-center gap-2 px-3 py-2">
              <span className="flex-1 text-sm text-text-primary truncate">{c.name}</span>
              <button onClick={() => renameCategory(c)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('edit')}><Pencil size={15} /></button>
              <button onClick={() => removeCategory(c)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
            </li>
          ))}
          {categories.length === 0 && <li className="px-3 py-3 text-sm text-text-tertiary">—</li>}
        </ul>
      </section>

      {/* Forums */}
      <section>
        <h2 className="text-sm font-semibold text-text-secondary mb-2">{t('forums')}</h2>
        <div className="flex items-end gap-2 mb-2">
          <div className="w-48">
            <Dropdown options={categories.map(c => ({ value: c.id, label: c.name }))} value={newForumCat} onChange={setNewForumCat} placeholder={t('category')} width="100%" height={36} />
          </div>
          <Input value={newForumName} onChange={(e) => setNewForumName(e.target.value)} placeholder={t('name')} className="flex-1" />
          <Button variant="secondary" icon={<Plus size={15} />} disabled={!newForumCat || !newForumName.trim()} onClick={addForum}>{t('add')}</Button>
        </div>
        <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
          {forums.map(f => (
            <li key={f.id} className="flex items-center gap-2 px-3 py-2">
              <MessagesSquare size={15} className="text-primary shrink-0" />
              <span className="flex-1 text-sm text-text-primary truncate">{f.name}</span>
              <button onClick={() => toggleLock(f)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={f.is_locked ? t('unlock_topic') : t('lock_topic')}>
                {f.is_locked ? <Lock size={15} /> : <Unlock size={15} />}
              </button>
              <button onClick={() => setModForum(f.id)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('moderators')}><Shield size={15} /></button>
              <button onClick={() => setPermForum(f.id)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('permissions')}><KeyRound size={15} /></button>
              <button onClick={() => renameForum(f)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('edit')}><Pencil size={15} /></button>
              <button onClick={() => removeForum(f)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
            </li>
          ))}
          {forums.length === 0 && <li className="px-3 py-3 text-sm text-text-tertiary">{t('no_forums')}</li>}
        </ul>
      </section>

      {modForum && <ModeratorsWindow forumId={modForum} onClose={() => setModForum(null)} />}
      {permForum && <PermissionsWindow forumId={permForum} onClose={() => setPermForum(null)} />}
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}

function RanksTab() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const { data: ranks = [], isLoading } = useQuery({ queryKey: ['forum-ranks'], queryFn: forumApi.listRanks })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['forum-ranks'] })

  const addRank = async () => {
    const title = await prompt({ title: t('new_rank'), placeholder: t('title'), confirmLabel: t('create') })
    if (!title?.trim()) return
    const minStr = await prompt({ title: t('min_posts'), placeholder: '0', confirmLabel: t('save') })
    await forumApi.createRank({ title: title.trim(), min_posts: Number(minStr) || 0 })
    invalidate()
  }
  const editRank = async (r: Rank) => {
    const minStr = await prompt({ title: t('min_posts'), defaultValue: String(r.min_posts), confirmLabel: t('save') })
    if (minStr === null) return
    await forumApi.updateRank(r.id, { min_posts: Number(minStr) || 0 })
    invalidate()
  }
  const removeRank = async (r: Rank) => {
    if (await confirm({ title: t('delete'), message: r.title, confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteRank(r.id); invalidate()
    }
  }

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  return (
    <div>
      <div className="flex justify-end mb-2">
        <Button variant="secondary" size="sm" icon={<Plus size={15} />} onClick={addRank}>{t('new_rank')}</Button>
      </div>
      <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
        {ranks.map(r => (
          <li key={r.id} className="flex items-center gap-2 px-3 py-2">
            <Award size={15} className="text-primary shrink-0" />
            <span className="flex-1 text-sm text-text-primary truncate">{r.title}</span>
            <span className="text-xs text-text-tertiary">{t('member_posts', { count: r.min_posts })}</span>
            {!r.is_special && <button onClick={() => editRank(r)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('edit')}><Pencil size={15} /></button>}
            <button onClick={() => removeRank(r)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
          </li>
        ))}
      </ul>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}

function ProfileTab() {
  const { t } = useTranslation('forum')
  const { data: profile, isLoading } = useQuery({ queryKey: ['forum-my-profile'], queryFn: forumApi.myProfile })
  const [sig, setSig] = useState('')
  const [saved, setSaved] = useState(false)

  useEffect(() => { if (profile) setSig(profile.signature_md ?? '') }, [profile])

  const save = async () => {
    await forumApi.updateMySignature(sig.trim() || null)
    setSaved(true); setTimeout(() => setSaved(false), 2000)
  }

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  return (
    <div className="space-y-3 max-w-xl">
      <div className="text-sm text-text-secondary">{t('member_posts', { count: profile?.post_count ?? 0 })}</div>
      <label className="block text-xs font-medium text-text-secondary">{t('my_signature')}</label>
      <PostEditor value={sig} onChange={setSig} rows={4} placeholder={t('signature')} />
      <div className="flex items-center gap-3">
        <Button variant="primary" onClick={save}>{t('save')}</Button>
        {saved && <span className="text-sm text-success">✓</span>}
      </div>
    </div>
  )
}
