// Bridge so non-component callbacks (e.g. the sidebar "New" menu items) can use
// the React Query client. The sidebar — mounted for every /forum route —
// registers the live instance here.
// (Navigation itself no longer needs a bridge: use `navigate` from @kubuno/sdk.)
import type { QueryClient } from '@tanstack/react-query'

let queryClient: QueryClient | null = null

export function setQueryClient(qc: QueryClient) {
  queryClient = qc
}

export function getQueryClient(): QueryClient | null {
  return queryClient
}
