/** Forum module bundle — loaded at runtime (cf. vite.config). */
import { lazy } from 'react'
import {
  RouteRegistry, WaffleAppRegistry,
  ExtensionRegistry,
  ModuleSettingsRegistry,
  NotificationRegistry,
  useSidebarStore, useSearchStore,
  navigate,
  SDK_VERSION,
} from '@kubuno/sdk'
import { MessagesSquare } from 'lucide-react'
import './index.css'
import './i18n'
import { useForumStore } from './store'
import { newActionItems } from './ForumCreateMenu'
import ForumSidebarBody from './ForumSidebarBody'
import { registerForumAdmin } from './admin/ForumAdminPanel'

export const sdkVersion = SDK_VERSION

export function register() {
  WaffleAppRegistry.register('forum', 'Forum', [
    { id: 'forum', label: 'Forum', Icon: MessagesSquare, path: '/forum' },
  ])

  // The header gear button opens the per-user Forum settings while in /forum.
  ModuleSettingsRegistry.register('forum')

  // Instance administration (structure, ranks) in the core admin console.
  registerForumAdmin()

  // Declare the notification activities shown in the core Settings → Notifications matrix.
  NotificationRegistry.register({
    moduleId: 'forum',
    title: 'Forum',
    order: 60,
    activities: [
      { id: 'topic_reply', label: 'Réponse à votre sujet', emailDefault: true, pushDefault: true },
      { id: 'mention', label: 'Vous êtes mentionné', emailDefault: true, pushDefault: true },
      { id: 'marked_solution', label: 'Votre message est marqué comme solution', emailDefault: true },
    ],
  })

  useSidebarStore.getState().register({
    moduleId:          'forum',
    routePrefix:       '/forum',
    newButtonLabelKey: 'forum:new_topic',
    SidebarBody:       ForumSidebarBody,
    collapsedBody:     true,
  })

  // Sidebar "New" button: contribute MenuItem[] (data) to the shell menu.
  ExtensionRegistry.register('shell.new-actions', 'forum', {
    moduleId: 'forum',
    items: newActionItems,
  })

  useSearchStore.getState().register({
    moduleId:       'forum',
    routePrefix:    '/forum',
    placeholder:    'Search the forum…',
    placeholderKey: 'forum:search_ph',
    onSearch:       (q) => { useForumStore.getState().setSearchQuery(q); navigate('/forum/search') },
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
