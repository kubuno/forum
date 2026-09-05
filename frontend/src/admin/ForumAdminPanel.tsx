// Instance administration of the forum, rendered in the core admin console under
// Modules ▸ Forum (slot `module-admin:forum`). It manages OBJECTS — categories,
// forums, ranks — not scalar settings, so each page is a custom section
// registered through `ModuleAdminRegistry`, exactly as mail registers its
// `addresses`/`dkim` sections. The backend endpoints are the same guarded ones
// (`assert_admin`) the module already exposed; only the surface moved here from
// the per-user settings page, where these tabs never belonged.

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  FolderPlus, MessagesSquare, Pencil, Trash2, Lock, Unlock, Shield, KeyRound, Plus, Award,
  ChevronUp, ChevronDown, UserPlus, Flag, Check, X, Eraser, Ban, IdCard, HelpCircle,
} from 'lucide-react'
import { Button, Input, Dropdown, Spinner, ConfirmDialog, FloatingWindow, Toggle, Textarea } from '@ui'
import { useConfirm, ModuleAdminRegistry } from '@kubuno/sdk'
import {
  forumApi, type Category, type Forum, type Rank, type UserBrief, type FaqEntry,
  type ProfileField, type ProfileFieldType, type ProfileFieldVisibility,
} from '../api'
import ModeratorsWindow from '../ModeratorsWindow'
import PermissionsWindow from '../PermissionsWindow'

// ── Formulaire d'édition complet d'un forum ──────────────────────────────────
//
// Replaces the old chain of single-field `prompt()` dialogs (name only) with a
// real form covering every column `UpdateForumDto` accepts: name, description,
// color, icon, rules, parent forum, position, locked, read-only.

function ForumEditWindow({ forum, forums, onClose }: { forum: Forum; forums: Forum[]; onClose: () => void }) {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const [name, setName] = useState(forum.name)
  const [description, setDescription] = useState(forum.description ?? '')
  const [color, setColor] = useState(forum.color ?? '')
  const [icon, setIcon] = useState(forum.icon ?? '')
  const [rulesMd, setRulesMd] = useState(forum.rules_md ?? '')
  const [parentForumId, setParentForumId] = useState(forum.parent_forum_id ?? '')
  const [position, setPosition] = useState(String(forum.position))
  const [isLocked, setIsLocked] = useState(forum.is_locked)
  const [isReadonly, setIsReadonly] = useState(forum.is_readonly)
  const [busy, setBusy] = useState(false)

  // A forum can never become its own parent — the only self-reference the
  // dropdown needs to rule out; deeper cycles are for the server to refuse.
  const parentOptions = [
    { value: '', label: t('no_parent_forum', { defaultValue: 'None' }) },
    ...forums.filter(f => f.id !== forum.id).map(f => ({ value: f.id, label: f.name })),
  ]

  const save = async () => {
    if (!name.trim() || busy) return
    setBusy(true)
    try {
      await forumApi.updateForum(forum.id, {
        name: name.trim(),
        description: description.trim() || null,
        color: color.trim() || null,
        icon: icon.trim() || null,
        rules_md: rulesMd.trim() || null,
        parent_forum_id: parentForumId || null,
        position: Number(position) || 0,
        is_locked: isLocked,
        is_readonly: isReadonly,
      })
      qc.invalidateQueries({ queryKey: ['forum-forums'] })
      qc.invalidateQueries({ queryKey: ['forum-all-forums'] })
      onClose()
    } finally {
      setBusy(false)
    }
  }

  return (
    <FloatingWindow
      title={t('edit_forum', { defaultValue: 'Edit forum' })}
      icon={<Pencil size={18} />}
      onClose={onClose}
      defaultWidth={520}
      defaultHeight={680}
    >
      <div className="flex flex-col gap-3 p-4 h-full overflow-auto">
        <Input label={t('name')} value={name} autoFocus onChange={(e) => setName(e.target.value)} />
        <Textarea label={t('description')} value={description} onChange={(e) => setDescription(e.target.value)} rows={2} />
        <div className="flex gap-2">
          <Input label={t('forum_color', { defaultValue: 'Color' })} value={color} onChange={(e) => setColor(e.target.value)} placeholder="#3b82f6" className="flex-1" />
          <Input label={t('forum_icon', { defaultValue: 'Icon' })} value={icon} onChange={(e) => setIcon(e.target.value)} placeholder="💬" className="flex-1" />
        </div>
        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1">{t('parent_forum', { defaultValue: 'Parent forum' })}</label>
          <Dropdown options={parentOptions} value={parentForumId} onChange={setParentForumId} width="100%" height={36} />
        </div>
        <Input label={t('position', { defaultValue: 'Position' })} type="number" value={position} onChange={(e) => setPosition(e.target.value)} />
        <Textarea
          label={t('forum_rules', { defaultValue: 'Rules' })}
          value={rulesMd}
          onChange={(e) => setRulesMd(e.target.value)}
          rows={5}
          placeholder={t('forum_rules_ph', { defaultValue: 'Markdown supported…' })}
        />
        <Toggle
          label={t('lock_forum', { defaultValue: 'Locked forum' })}
          description={t('lock_forum_desc', { defaultValue: 'No new topics or replies' })}
          checked={isLocked}
          onChange={(e) => setIsLocked(e.target.checked)}
        />
        <Toggle
          label={t('readonly_forum', { defaultValue: 'Read-only forum' })}
          description={t('readonly_forum_desc', { defaultValue: 'Visible to everyone, writable by no one' })}
          checked={isReadonly}
          onChange={(e) => setIsReadonly(e.target.checked)}
        />
        <div className="flex justify-end gap-2 pt-1 mt-auto">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} disabled={!name.trim()} onClick={save}>{t('save')}</Button>
        </div>
      </div>
    </FloatingWindow>
  )
}

// ── Structure : catégories + forums (instance-wide) ─────────────────────────────

function StructureSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const [modForum, setModForum] = useState<string | null>(null)
  const [permForum, setPermForum] = useState<string | null>(null)
  const [editForum, setEditForum] = useState<Forum | null>(null)
  const [newForumCat, setNewForumCat] = useState('')
  const [newForumName, setNewForumName] = useState('')
  const [addingCategory, setAddingCategory] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState('')
  const [editingCategoryId, setEditingCategoryId] = useState<string | null>(null)
  const [editCategoryName, setEditCategoryName] = useState('')

  const { data: categories = [], isLoading: lc } = useQuery({ queryKey: ['forum-categories'], queryFn: forumApi.listCategories })
  const { data: forums = [], isLoading: lf } = useQuery({ queryKey: ['forum-forums'], queryFn: () => forumApi.listForums() })

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ['forum-categories'] })
    qc.invalidateQueries({ queryKey: ['forum-forums'] })
    qc.invalidateQueries({ queryKey: ['forum-all-forums'] })
  }

  // In-place forms replace the old chained `prompt()` dialogs: an "adding" row
  // opens inline above the list, and renaming swaps the row's label for an
  // editable field — same interaction as `TopicView`'s post editing.
  const startAddCategory = () => { setAddingCategory(true); setNewCategoryName('') }
  const cancelAddCategory = () => { setAddingCategory(false); setNewCategoryName('') }
  const submitAddCategory = async () => {
    if (!newCategoryName.trim()) return
    await forumApi.createCategory({ name: newCategoryName.trim() })
    cancelAddCategory(); invalidate()
  }
  const startRenameCategory = (c: Category) => { setEditingCategoryId(c.id); setEditCategoryName(c.name) }
  const cancelRenameCategory = () => setEditingCategoryId(null)
  const submitRenameCategory = async () => {
    if (!editCategoryName.trim() || !editingCategoryId) return
    await forumApi.updateCategory(editingCategoryId, { name: editCategoryName.trim() })
    cancelRenameCategory(); invalidate()
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
  const removeForum = async (f: Forum) => {
    if (await confirm({ title: t('delete_forum'), message: t('confirm_delete_forum'), confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteForum(f.id); invalidate()
    }
  }
  const toggleLock = async (f: Forum) => { await forumApi.updateForum(f.id, { is_locked: !f.is_locked }); invalidate() }

  const forumsOf = (categoryId: string) =>
    forums.filter(f => f.category_id === categoryId).sort((a, b) => a.position - b.position || a.name.localeCompare(b.name))

  // Reorders within ONE category: the ids sent are only that category's
  // forums, so the server only touches their `position` (see
  // `ForumService::reorder`) — every other category keeps its own ordering.
  const moveForum = async (items: Forum[], index: number, dir: -1 | 1) => {
    const j = index + dir
    if (j < 0 || j >= items.length) return
    const ids = items.map(f => f.id)
    ;[ids[index], ids[j]] = [ids[j], ids[index]]
    await forumApi.reorderForums(ids)
    qc.invalidateQueries({ queryKey: ['forum-all-forums'] })
    qc.invalidateQueries({ queryKey: ['forum-forums'] })
  }

  if (lc || lf) return <div className="py-10 flex justify-center"><Spinner /></div>

  return (
    <div className="space-y-5">
      {/* Categories */}
      <section>
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-sm font-semibold text-text-secondary">{t('categories')}</h2>
          {!addingCategory && (
            <Button variant="secondary" size="sm" icon={<FolderPlus size={15} />} onClick={startAddCategory}>{t('new_category')}</Button>
          )}
        </div>
        <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
          {addingCategory && (
            <li className="flex items-center gap-2 px-3 py-2">
              <Input
                autoFocus
                value={newCategoryName}
                onChange={(e) => setNewCategoryName(e.target.value)}
                placeholder={t('name')}
                className="flex-1"
                onKeyDown={(e) => { if (e.key === 'Enter') submitAddCategory(); if (e.key === 'Escape') cancelAddCategory() }}
              />
              <button onClick={cancelAddCategory} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('cancel')}><X size={15} /></button>
              <button onClick={submitAddCategory} disabled={!newCategoryName.trim()} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary disabled:opacity-30 disabled:pointer-events-none" title={t('create')}><Check size={15} /></button>
            </li>
          )}
          {categories.map(c => (
            <li key={c.id} className="flex items-center gap-2 px-3 py-2">
              {editingCategoryId === c.id ? (
                <>
                  <Input
                    autoFocus
                    value={editCategoryName}
                    onChange={(e) => setEditCategoryName(e.target.value)}
                    className="flex-1"
                    onKeyDown={(e) => { if (e.key === 'Enter') submitRenameCategory(); if (e.key === 'Escape') cancelRenameCategory() }}
                  />
                  <button onClick={cancelRenameCategory} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('cancel')}><X size={15} /></button>
                  <button onClick={submitRenameCategory} disabled={!editCategoryName.trim()} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary disabled:opacity-30 disabled:pointer-events-none" title={t('save')}><Check size={15} /></button>
                </>
              ) : (
                <>
                  <span className="flex-1 text-sm text-text-primary truncate">{c.name}</span>
                  <button onClick={() => startRenameCategory(c)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('edit')}><Pencil size={15} /></button>
                  <button onClick={() => removeCategory(c)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
                </>
              )}
            </li>
          ))}
          {categories.length === 0 && !addingCategory && <li className="px-3 py-3 text-sm text-text-tertiary">—</li>}
        </ul>
      </section>

      {/* Forums, grouped by category so "move up/down" has an unambiguous
          neighbour to swap with. */}
      <section>
        <h2 className="text-sm font-semibold text-text-secondary mb-2">{t('forums')}</h2>
        <div className="flex items-end gap-2 mb-3">
          <div className="w-48">
            <Dropdown options={categories.map(c => ({ value: c.id, label: c.name }))} value={newForumCat} onChange={setNewForumCat} placeholder={t('category')} width="100%" height={36} />
          </div>
          <Input value={newForumName} onChange={(e) => setNewForumName(e.target.value)} placeholder={t('name')} className="flex-1" />
          <Button variant="secondary" icon={<Plus size={15} />} disabled={!newForumCat || !newForumName.trim()} onClick={addForum}>{t('add')}</Button>
        </div>

        {categories.map(c => {
          const items = forumsOf(c.id)
          return (
            <div key={c.id} className="mb-3">
              <h3 className="text-xs font-semibold text-text-tertiary uppercase tracking-wide mb-1">{c.name}</h3>
              <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
                {items.map((f, i) => (
                  <li key={f.id} className="flex items-center gap-2 px-3 py-2">
                    <div className="flex flex-col -my-1 shrink-0">
                      <button
                        onClick={() => moveForum(items, i, -1)}
                        disabled={i === 0}
                        className="p-0.5 rounded hover:bg-surface-1 text-text-tertiary disabled:opacity-30 disabled:pointer-events-none"
                        title={t('move_up', { defaultValue: 'Move up' })}
                      ><ChevronUp size={13} /></button>
                      <button
                        onClick={() => moveForum(items, i, 1)}
                        disabled={i === items.length - 1}
                        className="p-0.5 rounded hover:bg-surface-1 text-text-tertiary disabled:opacity-30 disabled:pointer-events-none"
                        title={t('move_down', { defaultValue: 'Move down' })}
                      ><ChevronDown size={13} /></button>
                    </div>
                    <MessagesSquare size={15} className="text-primary shrink-0" />
                    <span className="flex-1 text-sm text-text-primary truncate">{f.name}</span>
                    <button onClick={() => toggleLock(f)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={f.is_locked ? t('unlock_topic') : t('lock_topic')}>
                      {f.is_locked ? <Lock size={15} /> : <Unlock size={15} />}
                    </button>
                    <button onClick={() => setModForum(f.id)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('moderators')}><Shield size={15} /></button>
                    <button onClick={() => setPermForum(f.id)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('permissions')}><KeyRound size={15} /></button>
                    <button onClick={() => setEditForum(f)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('edit')}><Pencil size={15} /></button>
                    <button onClick={() => removeForum(f)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
                  </li>
                ))}
                {items.length === 0 && <li className="px-3 py-3 text-sm text-text-tertiary">{t('no_forums')}</li>}
              </ul>
            </div>
          )
        })}
        {categories.length === 0 && <p className="text-sm text-text-tertiary">{t('no_forums')}</p>}
      </section>

      {modForum && <ModeratorsWindow forumId={modForum} onClose={() => setModForum(null)} />}
      {permForum && <PermissionsWindow forumId={permForum} onClose={() => setPermForum(null)} />}
      {editForum && <ForumEditWindow forum={editForum} forums={forums} onClose={() => setEditForum(null)} />}
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}

// ── Purge des sujets inactifs (par forum) ────────────────────────────────────
//
// Bulk maintenance tool: soft-deletes every ordinary, non-pinned, non-solved,
// poll-less topic of a forum whose last activity is older than N days. The
// preview (`dry_run: true`) never writes anything — it only counts — so the
// admin sees the impact before committing, and the actual purge still runs
// through `useConfirm` (never the browser's `confirm()`).

function MaintenanceSection() {
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

// ── Assignation manuelle d'un rang spécial ───────────────────────────────────

function AssignRankForm() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
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

function RanksSection() {
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

// ── Raisons de signalement prédéfinies (instance-wide) ───────────────────────

function ReportReasonsSection() {
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

// ── Censure de mots (instance-wide, appliquée côté serveur) ─────────────────
//
// Every word/phrase here is substituted server-side, at render time, in every
// post body (`CensorService`, see `handlers/posts.rs::list`) — the admin's
// list is never sent to the client to filter locally, so there is nothing
// for a member to bypass by inspecting the response.

function CensoredWordsSection() {
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

// ── Custom profile fields (EAV) ─────────────────────────────────────────────────────────────────────
//
// Every answer a member gives to one of these fields is shown on their
// profile as plain, escaped text — never Markdown/HTML (see ProfilePage.tsx)
// — so there is nothing here for the admin's `options` list or a member's own
// answer to inject. Editing happens IN PLACE, like every other section on
// this page: adding opens an inline form row, editing swaps a row's display
// for the same form pre-filled — never a modal.

const PROFILE_FIELD_TYPES: ProfileFieldType[] = ['text', 'textarea', 'bool', 'url', 'date', 'dropdown']

function profileFieldTypeLabel(t: TFunction, type: ProfileFieldType): string {
  const labels: Record<ProfileFieldType, string> = {
    text:     t('profile_field_type_text', { defaultValue: 'Texte court' }),
    textarea: t('profile_field_type_textarea', { defaultValue: 'Texte long' }),
    bool:     t('profile_field_type_bool', { defaultValue: 'Oui / Non' }),
    url:      t('profile_field_type_url', { defaultValue: 'Lien (URL)' }),
    date:     t('profile_field_type_date', { defaultValue: 'Date' }),
    dropdown: t('profile_field_type_dropdown', { defaultValue: 'Liste déroulante' }),
  }
  return labels[type]
}

interface ProfileFieldFormValues {
  key: string
  label: string
  field_type: ProfileFieldType
  options: string[]
  position: number
  visibility: ProfileFieldVisibility
  show_on_posts: boolean
  required: boolean
}

function ProfileFieldForm({ initial, onCancel, onSubmit, busy }: {
  initial?: ProfileField
  onCancel: () => void
  onSubmit: (values: ProfileFieldFormValues) => void
  busy: boolean
}) {
  const { t } = useTranslation('forum')
  const [key, setKey] = useState(initial?.key ?? '')
  const [label, setLabel] = useState(initial?.label ?? '')
  const [fieldType, setFieldType] = useState<ProfileFieldType>(initial?.field_type ?? 'text')
  const [optionsText, setOptionsText] = useState((initial?.options ?? []).join('\n'))
  const [visibility, setVisibility] = useState<ProfileFieldVisibility>(initial?.visibility ?? 'public')
  const [showOnPosts, setShowOnPosts] = useState(initial?.show_on_posts ?? false)
  const [required, setRequired] = useState(initial?.required ?? false)
  const [position, setPosition] = useState(String(initial?.position ?? 0))

  const keyValid = /^[a-z0-9_]{1,40}$/.test(key.trim())
  const optionLines = optionsText.split('\n').map(o => o.trim()).filter(Boolean)
  const valid = keyValid && label.trim().length > 0 && (fieldType !== 'dropdown' || optionLines.length > 0)

  const submit = () => {
    if (!valid || busy) return
    onSubmit({
      key: key.trim(),
      label: label.trim(),
      field_type: fieldType,
      options: fieldType === 'dropdown' ? optionLines : [],
      position: Number(position) || 0,
      visibility,
      show_on_posts: showOnPosts,
      required,
    })
  }

  const typeOptions = PROFILE_FIELD_TYPES.map(v => ({ value: v, label: profileFieldTypeLabel(t, v) }))
  const visibilityOptions: { value: ProfileFieldVisibility; label: string }[] = [
    { value: 'public',     label: t('profile_field_visibility_public', { defaultValue: 'Publique' }) },
    { value: 'registered', label: t('profile_field_visibility_registered', { defaultValue: 'Membres connectés' }) },
  ]

  return (
    <li className="flex flex-col gap-2 px-3 py-3">
      <div className="flex gap-2">
        <Input
          autoFocus
          value={key}
          onChange={(e) => setKey(e.target.value.toLowerCase())}
          placeholder={t('profile_field_key', { defaultValue: 'Clé (ex : discord)' })}
          className="flex-1"
        />
        <Input
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          placeholder={t('profile_field_label', { defaultValue: 'Libellé affiché' })}
          className="flex-1"
        />
      </div>
      <div className="flex gap-2">
        <div className="flex-1">
          <Dropdown options={typeOptions} value={fieldType} onChange={(v) => setFieldType(v as ProfileFieldType)} width="100%" height={36} />
        </div>
        <div className="flex-1">
          <Dropdown options={visibilityOptions} value={visibility} onChange={(v) => setVisibility(v as ProfileFieldVisibility)} width="100%" height={36} />
        </div>
        <Input
          type="number"
          value={position}
          onChange={(e) => setPosition(e.target.value)}
          className="w-20"
          title={t('position', { defaultValue: 'Position' })}
        />
      </div>
      {fieldType === 'dropdown' && (
        <Textarea
          value={optionsText}
          onChange={(e) => setOptionsText(e.target.value)}
          rows={3}
          placeholder={t('profile_field_options_ph', { defaultValue: 'Une option par ligne' })}
        />
      )}
      <div className="flex items-center gap-4">
        <Toggle
          label={t('profile_field_show_on_posts', { defaultValue: 'Afficher sous les messages' })}
          checked={showOnPosts}
          onChange={(e) => setShowOnPosts(e.target.checked)}
        />
        <Toggle
          label={t('required', { defaultValue: 'Obligatoire' })}
          checked={required}
          onChange={(e) => setRequired(e.target.checked)}
        />
      </div>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" icon={<X size={14} />} onClick={onCancel}>{t('cancel')}</Button>
        <Button variant="primary" size="sm" icon={<Check size={14} />} disabled={!valid} loading={busy} onClick={submit}>
          {initial ? t('save') : t('create')}
        </Button>
      </div>
    </li>
  )
}

function ProfileFieldsSection() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const { data: fields = [], isLoading } = useQuery({ queryKey: ['forum-profile-fields'], queryFn: forumApi.listProfileFields })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['forum-profile-fields'] })
  const [adding, setAdding] = useState(false)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const create = async (values: ProfileFieldFormValues) => {
    setBusy(true)
    try {
      await forumApi.createProfileField(values)
      setAdding(false)
      invalidate()
    } finally {
      setBusy(false)
    }
  }
  const update = async (id: string, values: ProfileFieldFormValues) => {
    setBusy(true)
    try {
      await forumApi.updateProfileField(id, values)
      setEditingId(null)
      invalidate()
    } finally {
      setBusy(false)
    }
  }
  const remove = async (f: ProfileField) => {
    if (await confirm({ title: t('delete'), message: f.label, confirmLabel: t('delete'), variant: 'danger' })) {
      await forumApi.deleteProfileField(f.id)
      invalidate()
    }
  }

  if (isLoading) return <div className="py-10 flex justify-center"><Spinner /></div>
  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-sm font-semibold text-text-secondary">{t('profile_fields', { defaultValue: 'Champs de profil' })}</h2>
        {!adding && (
          <Button variant="secondary" size="sm" icon={<Plus size={15} />} onClick={() => setAdding(true)}>
            {t('new_profile_field', { defaultValue: 'Nouveau champ' })}
          </Button>
        )}
      </div>
      <p className="text-xs text-text-tertiary mb-2">
        {t('profile_fields_desc', {
          defaultValue: "Proposés à tous les membres pour compléter leur profil. Les réponses s'affichent en texte brut, jamais en Markdown.",
        })}
      </p>
      <ul className="rounded-xl border border-border bg-surface-0 divide-y divide-border">
        {adding && <ProfileFieldForm onCancel={() => setAdding(false)} onSubmit={create} busy={busy} />}
        {fields.map(f => (
          editingId === f.id ? (
            <ProfileFieldForm key={f.id} initial={f} onCancel={() => setEditingId(null)} onSubmit={(values) => update(f.id, values)} busy={busy} />
          ) : (
            <li key={f.id} className="flex items-center gap-2 px-3 py-2">
              <IdCard size={15} className="text-primary shrink-0" />
              <div className="flex-1 min-w-0">
                <div className="text-sm text-text-primary truncate">{f.label}</div>
                <div className="text-xs text-text-tertiary font-mono truncate">{f.key} · {profileFieldTypeLabel(t, f.field_type)}</div>
              </div>
              {f.required && (
                <span className="text-[10px] uppercase tracking-wide text-danger shrink-0">{t('required', { defaultValue: 'Obligatoire' })}</span>
              )}
              <button onClick={() => setEditingId(f.id)} className="p-1.5 rounded hover:bg-surface-1 text-text-tertiary" title={t('edit')}><Pencil size={15} /></button>
              <button onClick={() => remove(f)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('delete')}><Trash2 size={15} /></button>
            </li>
          )
        ))}
        {fields.length === 0 && !adding && <li className="px-3 py-3 text-sm text-text-tertiary">{t('no_profile_fields', { defaultValue: 'Aucun champ de profil.' })}</li>}
      </ul>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}

// ── FAQ (admin-curated, read by every member) ──────────────────
//
// Every entry here is shown to members on the dedicated `/forum/faq` page
// (`FaqPage.tsx`), rendered through the same sanitized Markdown renderer as
// post bodies. Editing happens IN PLACE, like the other sections on this
// page: adding opens an inline form row, editing swaps a row's display for
// the same form pre-filled — never a modal.

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

function FaqSection() {
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

// Registers the forum's admin sections into the core console. Each is mounted on
// its declared `[[setting_groups]]` page WITHOUT a label — the section IS the
// page (it brings its own layout), rather than one tab among several.
export function registerForumAdmin() {
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'structure',
    group:     'structure',
    position:  10,
    Component: StructureSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'maintenance',
    group:     'structure',
    position:  20,
    Component: MaintenanceSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'ranks',
    group:     'ranks',
    position:  10,
    Component: RanksSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'report-reasons',
    group:     'moderation',
    position:  10,
    Component: ReportReasonsSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'censored-words',
    group:     'moderation',
    position:  20,
    Component: CensoredWordsSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'profile-fields',
    group:     'profiles',
    position:  10,
    Component: ProfileFieldsSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'faq',
    group:     'structure',
    position:  30,
    Component: FaqSection,
  })
}
