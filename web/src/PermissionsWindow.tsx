import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query'
import { KeyRound } from 'lucide-react'
import { FloatingWindow, Button, Spinner } from '@ui'
import { forumApi, type Permission, type GroupPermission } from './api'

type Role = 'guest' | 'user' | 'moderator'
const ROLES: Role[] = ['guest', 'user']
const CAPS: (keyof Pick<Permission, 'can_view' | 'can_post' | 'can_reply' | 'can_attach'>)[] =
  ['can_view', 'can_post', 'can_reply', 'can_attach']

const GROUP_CAP_DEFAULT: Pick<GroupPermission, 'can_view' | 'can_post' | 'can_reply' | 'can_attach'> =
  { can_view: false, can_post: false, can_reply: false, can_attach: false }

const DEFAULT: Record<Role, Pick<Permission, 'can_view' | 'can_post' | 'can_reply' | 'can_attach'>> = {
  guest:     { can_view: true, can_post: false, can_reply: false, can_attach: false },
  user:      { can_view: true, can_post: true, can_reply: true, can_attach: true },
  moderator: { can_view: true, can_post: true, can_reply: true, can_attach: true },
}

/** The "per group" section: additive grants that can open a restricted
 *  forum to a group, without ever revoking role-based access. Reuses the
 *  same plain checkbox toggles as the role table above. */
function GroupPermissionsSection({ forumId }: { forumId: string }) {
  const { t } = useTranslation('forum')
  const queryClient = useQueryClient()
  const { data: groups, isLoading: groupsLoading } = useQuery({
    queryKey: ['forum-groups'],
    queryFn: () => forumApi.listGroups(),
  })
  const { data: groupPerms, isLoading: permsLoading } = useQuery({
    queryKey: ['forum-group-permissions', forumId],
    queryFn: () => forumApi.listGroupPermissions(forumId),
  })
  const mutation = useMutation({
    mutationFn: (body: { group_id: string; can_view: boolean; can_post: boolean; can_reply: boolean; can_attach: boolean }) =>
      forumApi.setGroupPermission(forumId, body),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['forum-group-permissions', forumId] }),
  })

  if (groupsLoading || permsLoading) {
    return <div className="flex justify-center items-center py-4"><Spinner /></div>
  }

  const toggle = (groupId: string, cap: typeof CAPS[number]) => {
    const current = groupPerms?.find(p => p.group_id === groupId)
    const next = { ...GROUP_CAP_DEFAULT, ...(current ?? {}), [cap]: !(current?.[cap] ?? false) }
    mutation.mutate({ group_id: groupId, can_view: next.can_view, can_post: next.can_post, can_reply: next.can_reply, can_attach: next.can_attach })
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="text-xs font-medium text-text-primary">{t('group_permissions', { defaultValue: 'Permissions par groupe' })}</div>
      <div className="text-xs text-text-tertiary">
        {t('group_permissions_help', { defaultValue: "Les autorisations de groupe s'ajoutent aux autorisations de rôle : elles ouvrent un forum restreint à certains groupes, sans jamais retirer d'accès." })}
      </div>
      {!groups || groups.length === 0 ? (
        <div className="text-xs text-text-tertiary py-2">{t('no_groups', { defaultValue: 'Aucun groupe défini.' })}</div>
      ) : (
        <table className="w-full text-sm">
          <thead>
            <tr className="text-xs text-text-tertiary">
              <th className="text-left font-medium py-1">{t('group', { defaultValue: 'Groupe' })}</th>
              {CAPS.map(c => <th key={c} className="font-medium py-1">{t(c)}</th>)}
            </tr>
          </thead>
          <tbody>
            {groups.map(group => {
              const current = groupPerms?.find(p => p.group_id === group.id)
              return (
                <tr key={group.id} className="border-t border-border">
                  <td className="py-2 text-text-primary">{group.name}</td>
                  {CAPS.map(cap => (
                    <td key={cap} className="text-center">
                      <input
                        type="checkbox"
                        checked={current?.[cap] ?? false}
                        onChange={() => toggle(group.id, cap)}
                      />
                    </td>
                  ))}
                </tr>
              )
            })}
          </tbody>
        </table>
      )}
    </div>
  )
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
    <FloatingWindow title={t('permissions')} icon={<KeyRound size={18} />} onClose={onClose} defaultWidth={460} defaultHeight={560}>
      <div className="p-4 flex flex-col gap-3 h-full overflow-y-auto">
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
        <div className="border-t border-border pt-3">
          <GroupPermissionsSection forumId={forumId} />
        </div>
        <div className="flex justify-end gap-2 mt-auto">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button variant="primary" loading={busy} onClick={save}>{t('save')}</Button>
        </div>
      </div>
    </FloatingWindow>
  )
}
