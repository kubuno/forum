import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { KeyRound } from 'lucide-react'
import { FloatingWindow, Button, Spinner } from '@ui'
import { forumApi, type Permission } from './api'

type Role = 'guest' | 'user' | 'moderator'
const ROLES: Role[] = ['guest', 'user']
const CAPS: (keyof Pick<Permission, 'can_view' | 'can_post' | 'can_reply' | 'can_attach'>)[] =
  ['can_view', 'can_post', 'can_reply', 'can_attach']

const DEFAULT: Record<Role, Pick<Permission, 'can_view' | 'can_post' | 'can_reply' | 'can_attach'>> = {
  guest:     { can_view: true, can_post: false, can_reply: false, can_attach: false },
  user:      { can_view: true, can_post: true, can_reply: true, can_attach: true },
  moderator: { can_view: true, can_post: true, can_reply: true, can_attach: true },
}

export default function PermissionsWindow({ forumId, onClose }: { forumId: string; onClose: () => void }) {
  const { t } = useTranslation('forum')
  const { data, isLoading } = useQuery({ queryKey: ['forum-permissions', forumId], queryFn: () => forumApi.listPermissions(forumId) })
  const [state, setState] = useState<Record<Role, typeof DEFAULT.user>>(DEFAULT)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    if (!data) return
    const next = { ...DEFAULT }
    for (const role of ROLES) {
      const row = data.find(p => p.role === role)
      if (row) next[role] = { can_view: row.can_view, can_post: row.can_post, can_reply: row.can_reply, can_attach: row.can_attach }
    }
    setState(next)
  }, [data])

  const toggle = (role: Role, cap: typeof CAPS[number]) =>
    setState(s => ({ ...s, [role]: { ...s[role], [cap]: !s[role][cap] } }))

  const save = async () => {
    setBusy(true)
    try {
      for (const role of ROLES) {
        await forumApi.setPermission(forumId, { role, ...state[role] })
      }
      onClose()
    } finally { setBusy(false) }
  }

  return (
    <FloatingWindow title={t('permissions')} icon={<KeyRound size={18} />} onClose={onClose} defaultWidth={460} defaultHeight={300}>
      <div className="p-4 flex flex-col gap-3 h-full">
        {isLoading ? <div className="flex-1 flex justify-center items-center"><Spinner /></div> : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-xs text-text-tertiary">
                <th className="text-left font-medium py-1">{t('role')}</th>
                {CAPS.map(c => <th key={c} className="font-medium py-1">{t(c)}</th>)}
              </tr>
            </thead>
            <tbody>
              {ROLES.map(role => (
                <tr key={role} className="border-t border-border">
                  <td className="py-2 text-text-primary">{t(`role_${role}`)}</td>
                  {CAPS.map(cap => (
                    <td key={cap} className="text-center">
                      <input type="checkbox" checked={state[role][cap]} onChange={() => toggle(role, cap)} />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <div className="flex justify-end gap-2 mt-auto">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} onClick={save}>{t('save')}</Button>
        </div>
      </div>
    </FloatingWindow>
  )
}
