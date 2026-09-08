// Structure section of the forum admin console (Modules ▸ Forum ▸ Structure):
// categories and forums, instance-wide. Manages OBJECTS, so it is a custom
// section rather than scalar settings — see `registerForumAdmin`.

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  FolderPlus, MessagesSquare, Pencil, Trash2, Lock, Unlock, Shield, KeyRound, Plus,
  ChevronUp, ChevronDown, Check, X,
} from 'lucide-react'
import { Button, Input, Dropdown, Spinner, ConfirmDialog, FloatingWindow, Toggle, Textarea } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi, type Category, type Forum } from '../../api'
import ModeratorsWindow from '../../ModeratorsWindow'
import PermissionsWindow from '../../PermissionsWindow'

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

export function StructureSection() {
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
