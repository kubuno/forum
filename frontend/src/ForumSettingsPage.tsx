import React, { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  FolderPlus, MessagesSquare, Pencil, Trash2, Lock, Unlock, Shield, KeyRound, Plus, Award,
  ArrowLeft, Check, ExternalLink,
} from 'lucide-react'
import { Button, Input, Dropdown, Spinner, ConfirmDialog, Toggle, Radio } from '@ui'
import { prompt, useConfirm, useAuthStore } from '@kubuno/sdk'
import { forumApi, type Category, type Forum, type Rank } from './api'
import PostEditor from './PostEditor'
import ModeratorsWindow from './ModeratorsWindow'
import PermissionsWindow from './PermissionsWindow'
import { useModulePrefs } from './userPrefs'

// ── Per-user preferences (backend, cross-device via core users.preferences) ─────

interface ForumPrefs {
  defaultFeed:   string   // 'recent' | 'unanswered' | 'popular'
  topicsPerPage: string   // '20' | '30' | '50'
  markReadOnOpen: boolean
  notifyReplies:  boolean
  notifyMentions: boolean
  showSignatures: boolean
  postOrder:      string  // 'asc' | 'desc'
}

const DEFAULT_PREFS: ForumPrefs = {
  defaultFeed: 'recent', topicsPerPage: '30', markReadOnOpen: true,
  notifyReplies: true, notifyMentions: true, showSignatures: true,
  postOrder: 'asc',
}

// ── Mail-style layout helpers ───────────────────────────────────────────────────

function SettingsRow({ label, description, children }: {
  label: string; description?: string; children: React.ReactNode
}) {
  return (
    <div className="flex items-start gap-8 py-4 border-b border-[#e8eaed] last:border-0">
      <div className="w-60 flex-shrink-0">
        <p className="text-sm text-[#202124] font-normal">{label}</p>
        {description && <p className="text-xs text-text-tertiary mt-0.5 leading-relaxed">{description}</p>}
      </div>
      <div className="flex-1">{children}</div>
    </div>
  )
}

function RadioGroup({ options, value, onChange }: {
  options: { value: string; label: string }[]; value: string; onChange: (v: string) => void
}) {
  return (
    <div className="flex flex-col items-start gap-2">
      {options.map(opt => (
        <Radio key={opt.value} checked={value === opt.value} onChange={() => onChange(opt.value)} label={opt.label} />
      ))}
    </div>
  )
}

// ── Préférences tab (per-user) ──────────────────────────────────────────────────

function PreferencesTab() {
  const { t } = useTranslation('forum')
  const { prefs: saved, update } = useModulePrefs<ForumPrefs>('forum', DEFAULT_PREFS)
  const [prefs, setPrefs] = useState<ForumPrefs>(saved)
  const [savedFlag, setSavedFlag] = useState(false)
  const [busy, setBusy] = useState(false)

  const set = <K extends keyof ForumPrefs>(key: K, value: ForumPrefs[K]) =>
    setPrefs(p => ({ ...p, [key]: value }))

  const save = async () => {
    setBusy(true)
    try {
      await update(prefs)
      setSavedFlag(true)
      setTimeout(() => setSavedFlag(false), 2500)
    } finally { setBusy(false) }
  }

  return (
    <div>
      <SettingsRow
        label={t('forum_pref_default_feed', { defaultValue: 'Flux par défaut' })}
        description={t('forum_pref_default_feed_desc', { defaultValue: 'Flux affiché à l\'ouverture du forum.' })}
      >
        <RadioGroup
          value={prefs.defaultFeed}
          onChange={v => set('defaultFeed', v)}
          options={[
            { value: 'recent',     label: t('forum_pref_feed_recent',     { defaultValue: 'Sujets récents' }) },
            { value: 'unanswered', label: t('forum_pref_feed_unanswered', { defaultValue: 'Sans réponse' }) },
            { value: 'popular',    label: t('forum_pref_feed_popular',    { defaultValue: 'Populaires' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('forum_pref_topics_per_page', { defaultValue: 'Sujets par page' })}
        description={t('forum_pref_topics_per_page_desc', { defaultValue: 'Nombre de sujets affichés par page dans les listes.' })}
      >
        <RadioGroup
          value={prefs.topicsPerPage}
          onChange={v => set('topicsPerPage', v)}
          options={[
            { value: '20', label: t('forum_pref_topics_count', { defaultValue: '{{count}} sujets', count: 20 }) },
            { value: '30', label: t('forum_pref_topics_count', { defaultValue: '{{count}} sujets', count: 30 }) },
            { value: '50', label: t('forum_pref_topics_count', { defaultValue: '{{count}} sujets', count: 50 }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('forum_pref_post_order', { defaultValue: 'Ordre des messages' })}
        description={t('forum_pref_post_order_desc', { defaultValue: 'Sens de lecture des messages dans un sujet.' })}
      >
        <RadioGroup
          value={prefs.postOrder}
          onChange={v => set('postOrder', v)}
          options={[
            { value: 'asc',  label: t('forum_pref_post_order_asc',  { defaultValue: 'Du plus ancien au plus récent' }) },
            { value: 'desc', label: t('forum_pref_post_order_desc_opt', { defaultValue: 'Du plus récent au plus ancien' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('forum_pref_mark_read', { defaultValue: 'Marquer comme lu' })}>
        <label className="flex items-center gap-2 cursor-pointer select-none">
          <Toggle checked={prefs.markReadOnOpen} onChange={() => set('markReadOnOpen', !prefs.markReadOnOpen)} />
          <span className="text-sm text-text-primary">{t('forum_pref_mark_read_on', { defaultValue: 'Marquer un sujet comme lu à son ouverture' })}</span>
        </label>
      </SettingsRow>

      <SettingsRow
        label={t('forum_pref_notify', { defaultValue: 'Notifications' })}
        description={t('forum_pref_notify_desc', { defaultValue: 'Recevoir des notifications du forum.' })}
      >
        <div className="flex flex-col gap-2">
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <Toggle checked={prefs.notifyReplies} onChange={() => set('notifyReplies', !prefs.notifyReplies)} />
            <span className="text-sm text-text-primary">{t('forum_pref_notify_replies', { defaultValue: 'Réponses à mes sujets' })}</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <Toggle checked={prefs.notifyMentions} onChange={() => set('notifyMentions', !prefs.notifyMentions)} />
            <span className="text-sm text-text-primary">{t('forum_pref_notify_mentions', { defaultValue: 'Mentions de mon nom' })}</span>
          </label>
        </div>
      </SettingsRow>

      <SettingsRow label={t('forum_pref_signatures', { defaultValue: 'Signatures' })}>
        <label className="flex items-center gap-2 cursor-pointer select-none">
          <Toggle checked={prefs.showSignatures} onChange={() => set('showSignatures', !prefs.showSignatures)} />
          <span className="text-sm text-text-primary">{t('forum_pref_signatures_on', { defaultValue: 'Afficher les signatures sous les messages' })}</span>
        </label>
      </SettingsRow>

      <div className="pt-5 flex items-center gap-3">
        <Button onClick={save} loading={busy}>
          {savedFlag
            ? <><Check size={14} className="mr-1.5 inline" />{t('forum_settings_saved', { defaultValue: 'Enregistré' })}</>
            : t('forum_settings_save_changes', { defaultValue: 'Enregistrer les modifications' })}
        </Button>
        <Button variant="ghost" onClick={() => setPrefs(saved)}>
          {t('cancel', { defaultValue: 'Annuler' })}
        </Button>
      </div>
    </div>
  )
}

// ── Structure tab (admin-only, instance-wide) ───────────────────────────────────

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

// ── Ranks tab (admin-only) ──────────────────────────────────────────────────────

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

// ── Profile tab (per-user) ──────────────────────────────────────────────────────

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

// ── About tab ───────────────────────────────────────────────────────────────────

function AboutTab() {
  const { t } = useTranslation('forum')
  return (
    <div className="rounded-xl border border-border overflow-hidden">
      <div className="flex items-center gap-3 px-5 py-4 border-b border-border bg-surface-1">
        <div className="w-10 h-10 rounded-xl bg-indigo-100 flex items-center justify-center shrink-0">
          <MessagesSquare size={20} className="text-indigo-600" />
        </div>
        <div>
          <p className="text-sm font-semibold text-text-primary">Kubuno Forum</p>
          <p className="text-xs text-text-tertiary">v0.1.0 · {t('forum_official_module', { defaultValue: 'Module officiel' })}</p>
        </div>
        <span className="ml-auto text-xs font-medium px-2 py-0.5 rounded-full bg-orange-100 text-orange-700">Rust</span>
      </div>
      <div className="px-5 py-4">
        <a href="https://github.com/kubuno/forum" target="_blank" rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline">
          <ExternalLink size={13} /> github.com/kubuno/forum
        </a>
      </div>
    </div>
  )
}

// ── Main page (mail-style breadcrumb + tab bar) ─────────────────────────────────

type Tab = 'preferences' | 'profile' | 'structure' | 'ranks' | 'about'

export default function ForumSettingsPage() {
  const { t } = useTranslation('forum')
  const isAdmin = useAuthStore(s => s.user?.role === 'admin')
  const [tab, setTab] = useState<Tab>('preferences')

  // Admin-only tabs (instance-wide structure/ranks) are hidden for non-admins.
  const tabs: { id: Tab; label: string; adminOnly?: boolean }[] = [
    { id: 'preferences', label: t('forum_tab_preferences', { defaultValue: 'Préférences' }) },
    { id: 'profile',     label: t('profile', { defaultValue: 'Profil' }) },
    { id: 'structure',   label: t('manage_structure', { defaultValue: 'Structure' }), adminOnly: true },
    { id: 'ranks',       label: t('ranks', { defaultValue: 'Rangs' }), adminOnly: true },
    { id: 'about',       label: t('forum_tab_about', { defaultValue: 'À propos' }) },
  ]
  const visibleTabs = tabs.filter(tb => !tb.adminOnly || isAdmin)

  return (
    <div className="flex flex-col h-full bg-white overflow-hidden">
      {/* Breadcrumb header */}
      <div className="flex items-center gap-2 px-6 py-2.5 border-b border-[#e8eaed] flex-shrink-0" style={{ background: '#f8f9fa' }}>
        <Link to="/forum" className="flex items-center gap-1.5 text-sm text-[#1a73e8] hover:underline">
          <ArrowLeft size={14} />
          Forum
        </Link>
        <span className="text-text-tertiary text-sm">/</span>
        <div className="flex items-center gap-1.5">
          <MessagesSquare size={15} className="text-text-secondary" />
          <span className="text-sm text-text-primary">{t('settings', { defaultValue: 'Réglages' })}</span>
        </div>
      </div>

      {/* Tab bar (Gmail-style) */}
      <div className="flex items-end border-b border-[#e8eaed] px-4 flex-shrink-0 overflow-x-auto" style={{ background: '#fff' }}>
        {visibleTabs.map(tb => (
          <button key={tb.id} onClick={() => setTab(tb.id)}
            className={`px-4 py-3 text-sm border-b-2 -mb-px transition-colors whitespace-nowrap ${
              tab === tb.id ? 'border-[#1a73e8] text-[#1a73e8] font-medium' : 'border-transparent text-[#5f6368] hover:text-[#202124] hover:bg-[#f1f3f4]'}`}>
            {tb.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl mx-auto px-8 py-6">
          {tab === 'preferences'           && <PreferencesTab />}
          {tab === 'profile'               && <ProfileTab />}
          {tab === 'structure' && isAdmin  && <StructureTab />}
          {tab === 'ranks'     && isAdmin  && <RanksTab />}
          {tab === 'about'                 && <AboutTab />}
        </div>
      </div>
    </div>
  )
}
