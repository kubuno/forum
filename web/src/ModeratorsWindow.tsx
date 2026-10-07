import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Shield, Trash2, UserPlus } from 'lucide-react'
import { FloatingWindow, Input, Spinner } from '@ui'
import { forumApi, type UserBrief } from './api'
import { useResolveUsers } from './users'
import { AuthorName } from './Author'

export default function ModeratorsWindow({ forumId, onClose }: { forumId: string; onClose: () => void }) {
  const { t } = useTranslation('forum')
  const qc = useQueryClient()
  const [q, setQ] = useState('')
  const [found, setFound] = useState<UserBrief[]>([])

  const { data: moderators = [], isLoading } = useQuery({
    queryKey: ['forum-moderators', forumId],
    queryFn: () => forumApi.listModerators(forumId),
  })
  useResolveUsers(moderators.map(m => m.user_id))

  const search = async (value: string) => {
    setQ(value)
    if (value.trim().length < 2) { setFound([]); return }
    setFound(await forumApi.searchUsers(value.trim()))
  }

  const add = async (uid: string) => {
    await forumApi.addModerator(forumId, uid)
    qc.invalidateQueries({ queryKey: ['forum-moderators', forumId] })
    setQ(''); setFound([])
  }
  const remove = async (uid: string) => {
    await forumApi.removeModerator(forumId, uid)
    qc.invalidateQueries({ queryKey: ['forum-moderators', forumId] })
  }

  return (
    <FloatingWindow title={t('moderators')} icon={<Shield size={18} />} onClose={onClose} defaultWidth={420} defaultHeight={420}>
      <div className="p-4 flex flex-col gap-3 h-full">
        <div className="relative">
          <Input value={q} onChange={(e) => search(e.target.value)} leftIcon={<UserPlus size={16} />} placeholder={t('add_moderator')} />
          {found.length > 0 && (
            <ul className="absolute z-10 left-0 right-0 mt-1 bg-surface-0 border border-border rounded-lg shadow-lg max-h-48 overflow-auto">
              {found.map(u => (
                <li key={u.id}>
                  <button onClick={() => add(u.id)} className="w-full text-left px-3 py-2 text-sm hover:bg-surface-1">
                    {u.display_name || u.username}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="flex-1 min-h-0 overflow-auto rounded-lg border border-border">
          {isLoading ? <div className="p-6 flex justify-center"><Spinner /></div>
            : moderators.length === 0 ? <div className="p-6 text-center text-sm text-text-tertiary">—</div>
            : (
              <ul className="divide-y divide-border">
                {moderators.map(m => (
                  <li key={m.user_id} className="flex items-center gap-2 px-3 py-2">
                    <span className="flex-1 text-sm text-text-primary"><AuthorName id={m.user_id} /></span>
                    <button onClick={() => remove(m.user_id)} className="p-1.5 rounded hover:bg-danger-light text-text-tertiary hover:text-danger" title={t('remove_moderator')}>
                      <Trash2 size={15} />
                    </button>
                  </li>
                ))}
              </ul>
            )}
        </div>
      </div>
    </FloatingWindow>
  )
}
