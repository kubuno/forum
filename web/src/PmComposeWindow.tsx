import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { X, Send } from 'lucide-react'
import { FloatingWindow, Button, Input } from '@ui'
import { forumApi, type UserBrief } from './api'
import PostEditor from './PostEditor'

interface Props {
  onClose: () => void
}

/** Compose a brand-new private conversation: pick recipients, optional
 *  subject, and a Markdown body — mirrors NewTopicWindow's shape. */
export default function PmComposeWindow({ onClose }: Props) {
  const { t } = useTranslation('forum')
  const navigate = useNavigate()
  const [query, setQuery] = useState('')
  const [found, setFound] = useState<UserBrief[]>([])
  const [recipients, setRecipients] = useState<UserBrief[]>([])
  const [subject, setSubject] = useState('')
  const [body, setBody] = useState('')
  const [busy, setBusy] = useState(false)

  const search = async (value: string) => {
    setQuery(value)
    if (value.trim().length < 2) { setFound([]); return }
    const results = await forumApi.searchUsers(value.trim())
    setFound(results.filter(u => !recipients.some(r => r.id === u.id)))
  }

  const addRecipient = (u: UserBrief) => {
    setRecipients(prev => [...prev, u])
    setQuery('')
    setFound([])
  }
  const removeRecipient = (id: string) => setRecipients(prev => prev.filter(r => r.id !== id))

  const submit = async () => {
    if (recipients.length === 0 || !body.trim() || busy) return
    setBusy(true)
    try {
      const { thread } = await forumApi.createPmThread({
        recipient_ids: recipients.map(r => r.id),
        subject: subject.trim() || undefined,
        body_md: body.trim(),
      })
      onClose()
      navigate(`/forum/pm/${thread.id}`)
    } finally {
      setBusy(false)
    }
  }

  return (
    <FloatingWindow
      title={t('new_message', { defaultValue: 'Nouveau message' })}
      icon={<Send size={18} />}
      onClose={onClose}
      defaultWidth={560}
      defaultHeight={540}
    >
      <div className="flex flex-col gap-3 p-4 h-full">
        <div>
          <label className="block text-xs font-medium text-text-secondary mb-1">
            {t('recipients', { defaultValue: 'Destinataires' })}
          </label>
          {recipients.length > 0 && (
            <div className="flex flex-wrap gap-1.5 mb-1.5">
              {recipients.map(r => (
                <span key={r.id} className="inline-flex items-center gap-1 pl-2 pr-1 py-1 rounded-full bg-surface-2 text-xs text-text-primary">
                  {r.display_name || r.username}
                  <button type="button" onClick={() => removeRecipient(r.id)}
                    className="p-0.5 rounded-full hover:bg-surface-3 text-text-tertiary hover:text-text-primary">
                    <X size={12} />
                  </button>
                </span>
              ))}
            </div>
          )}
          <div className="relative">
            <Input
              value={query}
              onChange={(e) => search(e.target.value)}
              placeholder={t('search_recipients', { defaultValue: 'Rechercher un membre…' })}
            />
            {found.length > 0 && (
              <ul className="absolute z-10 left-0 right-0 mt-1 bg-surface-0 border border-border rounded-lg shadow-lg max-h-48 overflow-auto">
                {found.map(u => (
                  <li key={u.id}>
                    <button type="button" onClick={() => addRecipient(u)} className="w-full text-left px-3 py-2 text-sm hover:bg-surface-1">
                      {u.display_name || u.username}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        <Input
          label={t('subject', { defaultValue: 'Sujet' })}
          value={subject}
          onChange={(e) => setSubject(e.target.value)}
          placeholder={t('subject_optional', { defaultValue: 'Sujet (facultatif)' })}
        />

        <div className="flex-1 min-h-0 flex flex-col">
          <label className="block text-xs font-medium text-text-secondary mb-1">{t('message')}</label>
          <PostEditor value={body} onChange={setBody} placeholder={t('write_message', { defaultValue: 'Écrivez votre message…' })} rows={8} />
        </div>

        <div className="flex justify-end gap-2 pt-1">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} disabled={recipients.length === 0 || !body.trim()} onClick={submit}>
            {t('send')}
          </Button>
        </div>
      </div>
    </FloatingWindow>
  )
}
