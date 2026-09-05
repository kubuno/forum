import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Rss, Copy, Check, Trash2 } from 'lucide-react'
import { Spinner, Button, Input, ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { forumApi } from './api'

/** Absolute URL of a feed document, as a feed reader would fetch it — through
 *  the core proxy, so it is rooted at the current origin. */
function feedUrl(token: string, kind: 'atom' | 'rss'): string {
  return `${window.location.origin}/api/v1/forum/public/feeds/${encodeURIComponent(token)}/${kind}.xml`
}

/** `/forum/feeds` — mint and revoke personal RSS/Atom feed URLs. Each token is a
 *  secret capability: anyone holding the URL sees exactly what its owner may see
 *  (evaluated as an ordinary member), so it must be treated like a password. */
export default function FeedsPage() {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const [label, setLabel] = useState('')
  const [copied, setCopied] = useState<string | null>(null)

  const { data: tokens = [], isLoading } = useQuery({
    queryKey: ['forum-feed-tokens'],
    queryFn: forumApi.listFeedTokens,
  })

  const create = async () => {
    await forumApi.createFeedToken(label.trim() || undefined)
    setLabel('')
    qc.invalidateQueries({ queryKey: ['forum-feed-tokens'] })
  }

  const revoke = async (token: string) => {
    const ok = await confirm({
      title: t('revoke_feed', { defaultValue: 'Révoquer ce flux ?' }),
      message: t('confirm_revoke_feed', { defaultValue: 'Le lien cessera immédiatement de fonctionner.' }),
      confirmLabel: t('revoke', { defaultValue: 'Révoquer' }),
      variant: 'danger',
    })
    if (!ok) return
    await forumApi.revokeFeedToken(token)
    qc.invalidateQueries({ queryKey: ['forum-feed-tokens'] })
  }

  const copy = async (token: string) => {
    try {
      await navigator.clipboard.writeText(feedUrl(token, 'atom'))
      setCopied(token)
      setTimeout(() => setCopied(c => (c === token ? null : c)), 1500)
    } catch { /* clipboard unavailable — the field is selectable as a fallback */ }
  }

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-3xl mx-auto px-4 py-5">
        <div className="flex items-center gap-2 mb-1">
          <Rss size={18} className="text-accent" />
          <h1 className="text-lg font-semibold text-text-primary">{t('rss_feeds', { defaultValue: 'Flux RSS' })}</h1>
        </div>
        <p className="text-sm text-text-secondary mb-4">
          {t('rss_feeds_desc', { defaultValue: 'Créez un lien personnel et secret pour suivre les sujets récents dans votre lecteur de flux. Il montre exactement ce que vous pouvez voir — ne le partagez pas.' })}
        </p>

        <div className="flex items-end gap-2 mb-5">
          <div className="flex-1">
            <label className="block text-xs font-medium text-text-secondary mb-1">
              {t('feed_label', { defaultValue: 'Nom (facultatif)' })}
            </label>
            <Input value={label} onChange={e => setLabel(e.target.value)} placeholder={t('feed_label_ph', { defaultValue: 'Ex : Lecteur maison' })} maxLength={80} />
          </div>
          <Button variant="primary" icon={<Rss size={14} />} onClick={create}>
            {t('create_feed', { defaultValue: 'Créer un flux' })}
          </Button>
        </div>

        <div className="rounded-xl border border-border overflow-hidden bg-surface-0">
          {isLoading ? (
            <div className="py-16 flex items-center justify-center"><Spinner size="lg" /></div>
          ) : tokens.length === 0 ? (
            <div className="px-4 py-16 text-center text-text-secondary">
              {t('no_feeds', { defaultValue: 'Aucun flux pour l’instant.' })}
            </div>
          ) : (
            <ul className="divide-y divide-border">
              {tokens.map(tk => (
                <li key={tk.token} className="px-4 py-3">
                  <div className="flex items-center gap-3">
                    <div className="min-w-0 flex-1">
                      <div className="font-medium text-text-primary truncate">
                        {tk.label || t('feed_unnamed', { defaultValue: 'Flux sans nom' })}
                      </div>
                      <div className="mt-1 flex items-center gap-2">
                        <input
                          readOnly
                          value={feedUrl(tk.token, 'atom')}
                          onFocus={e => e.currentTarget.select()}
                          className="flex-1 min-w-0 text-xs font-mono bg-surface-1 border border-border rounded-md px-2 py-1 text-text-secondary"
                        />
                        <Button
                          variant="ghost"
                          size="sm"
                          icon={copied === tk.token ? <Check size={14} /> : <Copy size={14} />}
                          onClick={() => copy(tk.token)}
                        >
                          {copied === tk.token ? t('copied', { defaultValue: 'Copié' }) : t('copy', { defaultValue: 'Copier' })}
                        </Button>
                      </div>
                    </div>
                    <Button variant="ghost" size="sm" icon={<Trash2 size={14} />} onClick={() => revoke(tk.token)}>
                      {t('revoke', { defaultValue: 'Révoquer' })}
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
