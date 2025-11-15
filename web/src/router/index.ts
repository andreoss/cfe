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
      path: '/t/:id',
      name: 'topic',
      component: () => import('../views/TopicView.vue'),
      props: true,
    },
    {
      path: '/search',
      name: 'search',
      component: () => import('../views/SearchView.vue'),
    },
    {
      path: '/tag/:tag',
      name: 'tag',
      component: () => import('../views/TagView.vue'),
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
  ],
})

export default router
