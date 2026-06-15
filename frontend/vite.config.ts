import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { fileURLToPath, URL } from 'node:url'

/**
 * Build the Forum module as a standalone ESM bundle:
 *   npm run build  →  dist/{entry.js, entry.css, chunks/*}
 *
 * Every specifier provided by the host (react, zustand, i18next, @ui,
 * @kubuno/sdk…) is `external`: at runtime the host's import map resolves them to
 * its singleton instances. `entry.js` exports `register()` + `sdkVersion`.
 * `lucide-react`, `date-fns`, `react-markdown` are bundled (they consume the
 * shared React via the `react` external).
 */
const SHARED = new Set([
  'react', 'react-dom', 'react-dom/client',
  'react/jsx-runtime', 'react/jsx-dev-runtime',
  'react-router-dom', '@tanstack/react-query',
  'zustand', 'react-i18next', 'i18next',
  '@ui', '@kubuno/sdk', '@kubuno/drive',
  '@radix-ui/react-dropdown-menu',
])
const isExternal = (s: string) =>
  SHARED.has(s) || s.startsWith('@ui/') || s.startsWith('@kubuno/sdk/') || s.startsWith('@kubuno/drive/')

// The shared specifiers above are `external`: never bundled, resolved at runtime
// by the host import map. TYPES come from the npm packages @kubuno/sdk /
// @kubuno/ui / @kubuno/drive (see tsconfig.json `paths` for `@ui`, whose
// specifier differs from the package name).
export default defineConfig({
  base: './',
  plugins: [react(), tailwindcss()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    cssCodeSplit: false,
    rollupOptions: {
      input: fileURLToPath(new URL('./src/entry.ts', import.meta.url)),
      external: isExternal,
      preserveEntrySignatures: 'strict',
      output: {
        format: 'es',
        entryFileNames: 'entry.js',
        chunkFileNames: 'chunks/[name]-[hash].js',
        assetFileNames: (info: { name?: string }) =>
          info.name?.endsWith('.css') ? 'entry.css' : 'assets/[name][extname]',
      },
    },
  },
})
