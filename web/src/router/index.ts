import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: HomeView,
    },
    {
      path: '/register',
      name: 'register',
      component: () => import('../views/RegisterView.vue'),
    },
    {
      path: '/sign-in',
      name: 'sign-in',
      component: () => import('../views/SignInView.vue'),
    },
    {
      path: '/forgot-password',
      name: 'forgot-password',
      component: () => import('../views/ForgotPasswordView.vue'),
    },
    {
      path: '/activate',
      name: 'activate',
      component: () => import('../views/ActivateView.vue'),
    },
    {
      path: '/u/:username',
      name: 'profile',
      component: () => import('../views/ProfileView.vue'),
      props: true,
    },
    {
      path: '/s/:slug',
      name: 'section',
      component: () => import('../views/SectionView.vue'),
      props: true,
    },
    {
      path: '/s/:slug/groups',
      name: 'groups',
      component: () => import('../views/GroupsView.vue'),
      props: true,
    },
    {
      path: '/s/:slug/g/:group',
      name: 'group',
      component: () => import('../views/GroupView.vue'),
      props: true,
    },
    {
      path: '/t/:id',
      name: 'topic',
      component: () => import('../views/TopicView.vue'),
      props: true,
    },
    {
      path: '/t/:id/history',
      name: 'topicHistory',
      component: () => import('../views/TopicHistoryView.vue'),
      props: true,
    },
    {
      path: '/search',
      name: 'search',
      component: () => import('../views/SearchView.vue'),
    },
    {
      path: '/activity',
      name: 'activity',
      component: () => import('../views/ActivityView.vue'),
    },
    {
      path: '/tag/:tag',
      name: 'tag',
      component: () => import('../views/TagView.vue'),
      props: true,
    },
    {
      path: '/archive',
      name: 'archive',
      component: () => import('../views/ArchiveView.vue'),
    },
    {
      path: '/archive/:year/:month',
      name: 'archiveMonth',
      component: () => import('../views/ArchiveMonthView.vue'),
      props: true,
    },
    {
      path: '/notifications',
      name: 'notifications',
      component: () => import('../views/NotificationsView.vue'),
    },
    {
      path: '/bookmarks',
      name: 'bookmarks',
      component: () => import('../views/BookmarksView.vue'),
    },
    {
      path: '/watched',
      name: 'watched',
      component: () => import('../views/WatchedView.vue'),
    },
    {
      path: '/notes',
      name: 'notes',
      component: () => import('../views/NotesView.vue'),
    },
    {
      path: '/invitations',
      name: 'invitations',
      component: () => import('../views/InvitationsView.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('../views/SettingsView.vue'),
    },
    {
      path: '/reports',
      name: 'reports',
      component: () => import('../views/ReportsView.vue'),
    },
    {
      path: '/addresses',
      name: 'addresses',
      component: () => import('../views/AddressView.vue'),
    },
    {
      path: '/section-settings',
      name: 'sectionSettings',
      component: () => import('../views/SectionSettingsView.vue'),
    },
  ],
})

export default router
