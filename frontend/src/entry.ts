/** Forum module bundle — loaded at runtime (cf. vite.config). */
import { lazy } from 'react'
import {
  RouteRegistry, WaffleAppRegistry,
  useSidebarStore, useSearchStore,
  SDK_VERSION,
} from '@kubuno/sdk'
import { MessagesSquare } from 'lucide-react'
import './index.css'
import './i18n'
import { useForumStore } from './store'
import { goTo } from './nav'
import ForumCreateMenu from './ForumCreateMenu'
import ForumSidebarBody from './ForumSidebarBody'

export const sdkVersion = SDK_VERSION

export function register() {
  WaffleAppRegistry.register('forum', 'Forum', [
    { id: 'forum', label: 'Forum', Icon: MessagesSquare, path: '/forum' },
  ])

  useSidebarStore.getState().register({
    moduleId:          'forum',
    routePrefix:       '/forum',
    newButtonLabelKey: 'forum:new_topic',
    NewActions:        ForumCreateMenu,
    SidebarBody:       ForumSidebarBody,
    collapsedBody:     true,
  })

  useSearchStore.getState().register({
    moduleId:       'forum',
    routePrefix:    '/forum',
    placeholder:    'Search the forum…',
    placeholderKey: 'forum:search_ph',
    onSearch:       (q) => { useForumStore.getState().setSearchQuery(q); goTo('/forum/search') },
  })

  const CategoryIndex     = lazy(() => import('./CategoryIndex'))
  const ForumView         = lazy(() => import('./ForumView'))
  const TopicView         = lazy(() => import('./TopicView'))
  const SearchView        = lazy(() => import('./SearchView'))
  const FeedView          = lazy(() => import('./FeedView'))
  const ModerationPanel   = lazy(() => import('./ModerationPanel'))
  const ForumSettingsPage = lazy(() => import('./ForumSettingsPage'))

  RouteRegistry.register('forum',              CategoryIndex)
  RouteRegistry.register('forum/forums/:id',   ForumView)
  RouteRegistry.register('forum/topics/:id',   TopicView)
  RouteRegistry.register('forum/feed/:kind',   FeedView)
  RouteRegistry.register('forum/search',       SearchView)
  RouteRegistry.register('forum/moderation',   ModerationPanel)
  RouteRegistry.register('forum/settings',     ForumSettingsPage)
}
