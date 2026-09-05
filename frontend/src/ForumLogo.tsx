interface ForumLogoProps {
  size?:      number
  className?: string
  title?:     string
}

/** Forum logo (designer artwork, raster). Served by the host from
 *  `/forum-logo.png`; rendered as a square image so it weighs the same as its
 *  neighbours in the waffle menu. */
export function ForumLogo({ size = 24, className, title = 'Forum' }: ForumLogoProps) {
  return (
    <img
      src="/forum-logo.png"
      width={size}
      height={size}
      alt={title}
      className={className}
      style={{ display: 'block', objectFit: 'contain' }}
    />
  )
}

export default ForumLogo
