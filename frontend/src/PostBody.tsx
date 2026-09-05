import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeHighlight from 'rehype-highlight'
import './code-highlight.css'

// Allow only safe URL schemes on links and images. Anything else — javascript:,
// data:, vbscript:, … — is dropped, so a crafted `[x](javascript:…)` or
// `![x](data:…)` cannot execute script or smuggle an inline payload. Relative
// and fragment URLs resolve to the https base below and pass through unchanged.
// Enforced here rather than trusted from the library's defaults (SEC-17).
// NOTE: never add `rehype-raw` here without `rehype-sanitize` — raw HTML would
// reopen the XSS surface this renderer is designed to avoid. `rehype-highlight`
// below is safe alongside this: it only walks the already-sanitized hast tree
// produced from Markdown and wraps code tokens in `<span>`s, it never parses
// or injects raw HTML.
const SAFE_SCHEMES = ['http:', 'https:', 'mailto:', 'tel:']
function safeUrl(url: string): string {
  try {
    return SAFE_SCHEMES.includes(new URL(url, 'https://forum.invalid').protocol) ? url : ''
  } catch {
    return ''
  }
}

/**
 * Safe Markdown (GFM) rendering for a forum post: bold/italic, lists, links,
 * images, code, tables, emoji. react-markdown does NOT interpret raw HTML → no
 * XSS risk. Links open in a new tab, images are size-bounded.
 */
export default function PostBody({ body, signature }: { body: string; signature?: boolean }) {
  // A signature reuses this exact renderer — the single sanitized Markdown path —
  // just muted and smaller, so no second, riskier rendering route is introduced.
  const base = signature ? 'text-xs text-text-tertiary' : 'text-sm text-text-primary'
  return (
    <div className={`${base} break-words leading-relaxed
                    [&_p]:my-2 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5
                    [&_blockquote]:bg-surface-2 [&_blockquote]:rounded-lg [&_blockquote]:px-3 [&_blockquote]:py-2 [&_blockquote]:text-text-secondary
                    [&_h1]:text-lg [&_h1]:font-semibold [&_h2]:text-base [&_h2]:font-semibold [&_h3]:font-semibold
                    [&_pre]:bg-surface-2 [&_pre]:p-3 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-2
                    [&_table]:border-collapse [&_th]:border [&_th]:border-border [&_th]:px-2 [&_th]:py-1
                    [&_td]:border [&_td]:border-border [&_td]:px-2 [&_td]:py-1`}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        urlTransform={safeUrl}
        components={{
          a: ({ ...props }) => <a {...props} target="_blank" rel="noopener noreferrer" className="text-primary underline" />,
          img: ({ ...props }) => <img {...props} className="max-w-full max-h-96 rounded-lg my-2" loading="lazy" />,
          code: ({ className, ...props }) => {
            // Fenced code blocks are tagged `hljs` (plus `language-xxx` when
            // detected) by rehype-highlight — keep those classes so
            // code-highlight.css can color the tokens. Inline `code` spans
            // carry no such class and keep the small badge style as before.
            const isHighlighted = /(?:^|\s)hljs(?:\s|$)/.test(className ?? '')
            return isHighlighted
              ? <code {...props} className={className} />
              : <code {...props} className="bg-surface-2 rounded px-1 py-0.5 text-xs" />
          },
        }}
      >
        {body}
      </ReactMarkdown>
    </div>
  )
}
