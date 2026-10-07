// Custom profile fields section (Modules ▸ Forum ▸ Profiles), an admin-defined
// EAV set every member can fill in.
//
// Every answer a member gives to one of these fields is shown on their
// profile as plain, escaped text — never Markdown/HTML (see ProfilePage.tsx)
// — so there is nothing here for the admin's `options` list or a member's own
// answer to inject. Editing happens IN PLACE, like every other section on
// this page: adding opens an inline form row, editing swaps a row's display
// for the same form pre-filled — never a modal.

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, IdCard, Pencil, Trash2, Check, X } from 'lucide-react'
import { Button, Input, Dropdown, Spinner, ConfirmDialog, Toggle, Textarea } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import {
  forumApi, type ProfileField, type ProfileFieldType, type ProfileFieldVisibility,
} from '../../api'

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

export function ProfileFieldsSection() {
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
