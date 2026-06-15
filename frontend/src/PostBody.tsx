import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

/**
 * Safe Markdown (GFM) rendering for a forum post: bold/italic, lists, links,
 * images, code, tables, emoji. react-markdown does NOT interpret raw HTML → no
 * XSS risk. Links open in a new tab, images are size-bounded.
 */
export default function PostBody({ body }: { body: string }) {
  return (
    <div className="text-sm text-text-primary break-words leading-relaxed
                    [&_p]:my-2 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5
                    [&_blockquote]:border-l-4 [&_blockquote]:border-border [&_blockquote]:pl-3 [&_blockquote]:text-text-secondary
                    [&_h1]:text-lg [&_h1]:font-semibold [&_h2]:text-base [&_h2]:font-semibold [&_h3]:font-semibold
                    [&_pre]:bg-surface-2 [&_pre]:p-3 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-2
                    [&_table]:border-collapse [&_th]:border [&_th]:border-border [&_th]:px-2 [&_th]:py-1
                    [&_td]:border [&_td]:border-border [&_td]:px-2 [&_td]:py-1">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ ...props }) => <a {...props} target="_blank" rel="noopener noreferrer" className="text-primary underline" />,
          img: ({ ...props }) => <img {...props} className="max-w-full max-h-96 rounded-lg my-2" loading="lazy" />,
          code: ({ ...props }) => <code {...props} className="bg-surface-2 rounded px-1 py-0.5 text-xs" />,
        }}
      >
        {body}
      </ReactMarkdown>
    </div>
  )
}
