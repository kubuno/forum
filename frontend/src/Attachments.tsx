import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Paperclip, X, FileText, Download } from 'lucide-react'
import { useFilesDialogStore, filesApi, getFileIcon } from '@kubuno/drive'
import { forumApi } from './api'
import { formatBytes } from './helpers'

/** A file chosen from the drive, pending attachment to a not-yet-created post. */
export interface PendingAttachment {
  file_id: string
  filename: string
  mime_type: string
  size_bytes: number
}

const isImage = (mime: string | null | undefined) => !!mime && mime.startsWith('image/')

/** Composer control: pick files from the drive and review the pending list. */
export function AttachPicker({ value, onChange }: { value: PendingAttachment[]; onChange: (v: PendingAttachment[]) => void }) {
  const { t } = useTranslation('forum')

  const pick = async () => {
    const file = await useFilesDialogStore.getState().openFile({ title: t('attach') })
    if (!file) return
    if (value.some(v => v.file_id === file.id)) return
    onChange([...value, { file_id: file.id, filename: file.name, mime_type: file.mime_type, size_bytes: file.size_bytes }])
  }

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <button type="button" onClick={pick}
        className="flex items-center gap-1.5 px-2 py-1 rounded-md text-xs text-text-secondary hover:text-text-primary hover:bg-surface-2 border border-border">
        <Paperclip size={14} /> {t('attach')}
      </button>
      {value.map(a => (
        <span key={a.file_id} className="flex items-center gap-1 px-2 py-1 rounded-md text-xs bg-surface-1 text-text-primary">
          <FileText size={13} className="text-text-tertiary" />
          <span className="max-w-[160px] truncate">{a.filename}</span>
          <button type="button" onClick={() => onChange(value.filter(v => v.file_id !== a.file_id))}
            className="text-text-tertiary hover:text-danger"><X size={13} /></button>
        </span>
      ))}
    </div>
  )
}

/** Persist pending attachments once the post exists. */
export async function saveAttachments(postId: string, pending: PendingAttachment[]) {
  for (const a of pending) {
    await forumApi.createAttachment(postId, a)
  }
}

/** Read-only attachment list shown under a post. */
export function PostAttachments({ postId }: { postId: string }) {
  const { t } = useTranslation('forum')
  const { data: attachments = [] } = useQuery({
    queryKey: ['forum-attachments', postId],
    queryFn: () => forumApi.listAttachments(postId),
  })
  if (attachments.length === 0) return null

  return (
    <div className="mt-3 pt-3 border-t border-border">
      <div className="text-[11px] font-semibold uppercase tracking-wide text-text-tertiary mb-2">{t('attachments')}</div>
      <div className="flex flex-wrap gap-2">
        {attachments.map(a => a.file_id && isImage(a.mime_type) ? (
          <a key={a.id} href={filesApi.downloadUrl(a.file_id)} target="_blank" rel="noopener noreferrer" title={a.filename}>
            <img src={filesApi.thumbnailUrl(a.file_id)} alt={a.filename}
              className="w-24 h-24 object-cover rounded-lg border border-border hover:opacity-90" loading="lazy" />
          </a>
        ) : (
          <a key={a.id} href={a.file_id ? filesApi.downloadUrl(a.file_id) : '#'} target="_blank" rel="noopener noreferrer"
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg border border-border hover:bg-surface-1 text-sm">
            <span className="text-text-tertiary">{getFileIcon(a.mime_type ?? '', a.filename)}</span>
            <span className="max-w-[180px] truncate text-text-primary">{a.filename}</span>
            {a.size_bytes ? <span className="text-xs text-text-tertiary">{formatBytes(a.size_bytes)}</span> : null}
            <Download size={14} className="text-text-tertiary" />
          </a>
        ))}
      </div>
    </div>
  )
}
