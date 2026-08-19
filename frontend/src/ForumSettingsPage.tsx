import React, { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { MessagesSquare, ArrowLeft, Check, ExternalLink } from 'lucide-react'
import { Button, Spinner, Toggle, Radio } from '@ui'
import { forumApi } from './api'
import PostEditor from './PostEditor'
import { useModulePrefs } from './userPrefs'

// This page holds ONLY per-user settings: forum preferences and the member's own
// signature. Instance administration (categories, forums, ranks) lives in the
// core admin console under Modules ▸ Forum — see `admin/ForumAdminPanel.tsx` —
// never on a user's own settings page.

// ── Per-user preferences (backend, cross-device via core users.preferences) ─────

// `type`, not `interface`: only a type alias gets the implicit index signature
// that `useModulePrefs<T extends Record<string, unknown>>` requires.
type ForumPrefs = {
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

type Tab = 'preferences' | 'profile' | 'about'

export default function ForumSettingsPage() {
  const { t } = useTranslation('forum')
  const [tab, setTab] = useState<Tab>('preferences')

  // Every tab here is per-user; instance administration moved to the core admin
  // console, so there is no admin gating on this page.
  const tabs: { id: Tab; label: string }[] = [
    { id: 'preferences', label: t('forum_tab_preferences', { defaultValue: 'Préférences' }) },
    { id: 'profile',     label: t('profile', { defaultValue: 'Profil' }) },
    { id: 'about',       label: t('forum_tab_about', { defaultValue: 'À propos' }) },
  ]

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
        {tabs.map(tb => (
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
          {tab === 'preferences' && <PreferencesTab />}
          {tab === 'profile'     && <ProfileTab />}
          {tab === 'about'       && <AboutTab />}
        </div>
      </div>
    </div>
  )
}
