import { create } from 'zustand'

interface ForumState {
  searchQuery: string
  setSearchQuery: (q: string) => void
}

export const useForumStore = create<ForumState>((set) => ({
  searchQuery: '',
  setSearchQuery: (q) => set({ searchQuery: q }),
}))
