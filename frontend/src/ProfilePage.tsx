import { useEffect, useState, type ReactNode } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronLeft, MessageSquare, MessagesSquare, Heart, Globe, MapPin, Clock } from 'lucide-react'
import { Spinner, Input, Textarea, Toggle, Dropdown, Button } from '@ui'
import { useAuthStore } from '@kubuno/sdk'
import { forumApi, type ProfileField } from './api'
import { AuthorName, AuthorAvatar } from './Author'
import { timeAgo, shortDateTime } from './helpers'
import PostBody from './PostBody'

// A custom profile field's value is ALWAYS plain, escaped text — never
// Markdown/HTML, unlike the bio above — so a member's own answer can never
// inject anything into another member's view of it. `url` is the one type
// that becomes a link, and only when it is actually http(s): this mirrors
// `PostBody.tsx`'s `safeUrl` allow-list (restricted further here, since a
// profile field is a bare address, not a Markdown link).
function safeFieldUrl(value: string): string | null {
  return value.startsWith('http://') || value.startsWith('https://') ? value : null
}

function ProfileFieldValueDisplay({ field, value }: { field: ProfileField; value: string }) {
  const { t } = useTranslation('forum')
  if (field.field_type === 'url') {
    const url = safeFieldUrl(value)
    return url
      ? <a href={url} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline break-all">{value}</a>
      : <span className="break-words">{value}</span>
  }
  if (field.field_type === 'bool') {
    return <span>{value === 'true' ? t('profile_field_yes', { defaultValue: 'Oui' }) : t('profile_field_no', { defaultValue: 'Non' })}</span>
  }
  return <span className="whitespace-pre-wrap break-words">{value}</span>
}

/** Read-only display of another member's answers, on their profile. Only
 *  fields the viewer may see AND that actually have an answer are returned
 *  by the backend (`values_for_display`), so nothing to filter here. */
function ProfileFieldsDisplay({ uid }: { uid: string }) {
  const { t } = useTranslation('forum')
  const { data: fields = [] } = useQuery({
    queryKey: ['forum-user-profile-fields', uid],
    queryFn: () => forumApi.getUserProfileFields(uid),
    enabled: !!uid,
  })
  if (fields.length === 0) return null
  return (
    <div className="rounded-xl border border-border bg-surface-0 p-4 mt-3">
      <h2 className="text-xs font-semibold uppercase tracking-wide text-text-tertiary mb-2">{t('profile_fields', { defaultValue: 'Champs de profil' })}</h2>
      <dl className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {fields.map(({ field, value }) => (
          <div key={field.id} className="min-w-0">
            <dt className="text-xs text-text-tertiary">{field.label}</dt>
            <dd className="text-sm text-text-primary mt-0.5"><ProfileFieldValueDisplay field={field} value={value} /></dd>
          </div>
        ))}
      </dl>
    </div>
  )
}

function ProfileFieldEditor({ field, value, onChange }: { field: ProfileField; value: string; onChange: (v: string) => void }) {
  const { t } = useTranslation('forum')
  const label = field.required ? `${field.label} *` : field.label
  switch (field.field_type) {
    case 'textarea':
      return <Textarea label={label} value={value} onChange={(e) => onChange(e.target.value)} rows={3} />
    case 'bool':
      return <Toggle label={label} checked={value === 'true'} onChange={(e) => onChange(e.target.checked ? 'true' : 'false')} />
    case 'date':
      return <Input label={label} type="date" value={value} onChange={(e) => onChange(e.target.value)} />
    case 'dropdown':
      return (
        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1">{label}</label>
          <Dropdown
            options={[
              { value: '', label: t('profile_field_none', { defaultValue: '—' }) },
              ...(field.options ?? []).map(o => ({ value: o, label: o })),
            ]}
            value={value}
            onChange={onChange}
            width="100%"
            height={36}
          />
        </div>
      )
    case 'url':
      return <Input label={label} value={value} onChange={(e) => onChange(e.target.value)} placeholder="https://…" />
    default:
      return <Input label={label} value={value} onChange={(e) => onChange(e.target.value)} />
  }
}

/** In-place editor for the CALLER's own answers (never someone else's) — no
 *  separate "edit mode": the widgets themselves are the editor, pre-filled
 *  with whatever is already saved, like the rest of this admin/profile UI. */
function MyProfileFieldsEditor() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { data, isLoading } = useQuery({ queryKey: ['forum-my-profile-fields'], queryFn: forumApi.getMyProfileFields })
  const [edits, setEdits] = useState<Record<string, string>>({})
  const [dirty, setDirty] = useState(false)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    if (!data) return
    const next: Record<string, string> = {}
    for (const f of data.fields) {
      next[f.id] = data.values.find(v => v.field_id === f.id)?.value ?? ''
    }
    setEdits(next)
    setDirty(false)
  }, [data])

  const setValue = (id: string, value: string) => {
    setEdits(e => ({ ...e, [id]: value }))
    setDirty(true)
  }

  const save = async () => {
    if (!data || busy) return
    setBusy(true)
    try {
      await forumApi.setMyProfileFields(data.fields.map(f => ({ field_id: f.id, value: edits[f.id] ?? '' })))
      await qc.invalidateQueries({ queryKey: ['forum-my-profile-fields'] })
      qc.invalidateQueries({ queryKey: ['forum-user-profile-fields'] })
      setDirty(false)
    } finally {
      setBusy(false)
    }
  }

  if (isLoading || !data || data.fields.length === 0) return null
  const sortedFields = [...data.fields].sort((a, b) => a.position - b.position)

  return (
    <div className="rounded-xl border border-border bg-surface-0 p-4 mt-3">
      <h2 className="text-xs font-semibold uppercase tracking-wide text-text-tertiary mb-2">{t('my_profile_fields', { defaultValue: 'Mes champs personnalisés' })}</h2>
      <div className="flex flex-col gap-3">
        {sortedFields.map(f => (
          <ProfileFieldEditor key={f.id} field={f} value={edits[f.id] ?? ''} onChange={(v) => setValue(f.id, v)} />
        ))}
      </div>
      <div className="flex justify-end mt-3">
        <Button variant="primary" size="sm" disabled={!dirty} loading={busy} onClick={save}>{t('save')}</Button>
      </div>
    </div>
  )
}

export default function ProfilePage() {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const { uid = '' } = useParams()
  const me = useAuthStore(s => s.user)
  const isOwn = !!me && me.id === uid

  const { data, isLoading } = useQuery({
    queryKey: ['forum-profile', uid],
    queryFn: () => forumApi.getProfilePage(uid),
    enabled: !!uid,
  })
  const { data: briefs = [] } = useQuery({
    queryKey: ['forum-briefs', [uid]],
    queryFn: () => forumApi.getBriefProfiles([uid]),
    enabled: !!uid,
  })

  if (isLoading) return <div className="h-full flex items-center justify-center"><Spinner size="lg" /></div>
  if (!data) return null
  const { profile, topics } = data
  const brief = briefs[0]
  const rankLabel = brief?.custom_title || brief?.rank_title

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <button onClick={() => navigate(-1)} className="mb-3 inline-flex items-center gap-1 text-xs text-text-tertiary hover:text-text-primary">
          <ChevronLeft size={14} /> {t('back')}
        </button>

        {/* Identity card */}
        <div className="rounded-xl border border-border bg-surface-0 p-5 flex items-start gap-4">
          <AuthorAvatar id={uid} size={72} />
          <div className="min-w-0 flex-1">
            <h1 className="text-lg font-semibold text-text-primary truncate"><AuthorName id={uid} /></h1>
            {rankLabel && (
              <div className="text-sm text-text-secondary flex items-center gap-1 mt-0.5">
                {brief?.rank_badge && <span>{brief.rank_badge}</span>}{rankLabel}
              </div>
            )}
            <div className="flex flex-wrap gap-x-4 gap-y-1 mt-2 text-xs text-text-tertiary">
              <span>{t('member_since', { defaultValue: 'Member since' })} {shortDateTime(profile.created_at)}</span>
              {profile.last_seen_at && (
                <span className="flex items-center gap-1"><Clock size={12} />{t('last_seen', { defaultValue: 'Last seen' })} {timeAgo(profile.last_seen_at)}</span>
              )}
              {profile.location && <span className="flex items-center gap-1"><MapPin size={12} />{profile.location}</span>}
              {profile.website && (
                <a href={profile.website} target="_blank" rel="noopener noreferrer" className="flex items-center gap-1 text-primary hover:underline">
                  <Globe size={12} />{t('website', { defaultValue: 'Website' })}
                </a>
              )}
            </div>
          </div>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-3 mt-3">
          <Stat icon={<MessageSquare size={16} />} value={profile.post_count} label={t('posts')} />
          <Stat icon={<MessagesSquare size={16} />} value={profile.topic_count} label={t('topics')} />
          <Stat icon={<Heart size={16} />} value={profile.likes_received} label={t('likes_received', { defaultValue: 'Likes' })} />
        </div>

        {/* Bio */}
        {profile.bio_md && (
          <div className="rounded-xl border border-border bg-surface-0 p-4 mt-3">
            <h2 className="text-xs font-semibold uppercase tracking-wide text-text-tertiary mb-2">{t('about', { defaultValue: 'About' })}</h2>
            <PostBody body={profile.bio_md} />
          </div>
        )}

        {/* Custom profile fields: an editable form on your own profile, a
            plain read-only card on anyone else's. */}
        {isOwn ? <MyProfileFieldsEditor /> : <ProfileFieldsDisplay uid={uid} />}

        {/* Recent topics */}
        {topics.length > 0 && (
          <div className="rounded-xl border border-border bg-surface-0 mt-3 overflow-hidden">
            <h2 className="text-xs font-semibold uppercase tracking-wide text-text-tertiary px-4 pt-3">{t('recent_topics', { defaultValue: 'Recent topics' })}</h2>
            <ul className="divide-y divide-border mt-1">
              {topics.map(tp => (
                <li key={tp.id}>
                  <button onClick={() => navigate(`/forum/topics/${tp.id}`)} className="w-full flex items-center gap-2 px-4 py-2.5 text-left hover:bg-surface-1">
                    <MessageSquare size={14} className="text-text-tertiary shrink-0" />
                    <span className="truncate flex-1 text-sm text-text-primary">{tp.title}</span>
                    <span className="text-xs text-text-tertiary shrink-0">{timeAgo(tp.created_at)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  )
}

function Stat({ icon, value, label }: { icon: ReactNode; value: number; label: string }) {
  return (
    <div className="rounded-xl border border-border bg-surface-0 p-3 flex flex-col items-center gap-0.5 text-center">
      <span className="text-text-tertiary">{icon}</span>
      <span className="text-lg font-semibold text-text-primary tabular-nums">{value}</span>
      <span className="text-[11px] text-text-tertiary">{label}</span>
    </div>
  )
}
