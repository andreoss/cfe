<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { RouterLink, RouterView } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { getUnreadCount } from '@/api/client'

const auth = useAuthStore()
const unreadCount = ref(0)

async function loadUnreadCount() {
  const result = await getUnreadCount()
  if (result.ok) {
    unreadCount.value = result.value
  }
}

watch(() => auth.currentUser, (newUser) => {
  if (newUser) {
    loadUnreadCount()
  } else {
    unreadCount.value = 0
  }
})

onMounted(() => {
  auth.checkSession()
})
</script>

<template>
  <header>
    <nav>
      <RouterLink to="/">Home</RouterLink>
      <RouterLink to="/search">Search</RouterLink>
      <RouterLink to="/activity">Activity</RouterLink>
      <RouterLink to="/archive">Archive</RouterLink>
      <template v-if="auth.currentUser">
        <RouterLink to="/notifications">{{
          unreadCount > 0 ? `Notifications (${unreadCount})` : 'Notifications'
        }}</RouterLink>
        <RouterLink to="/bookmarks">Saved</RouterLink>
        <RouterLink to="/watched">Watched</RouterLink>
        <RouterLink to="/notes">Notes</RouterLink>
        <RouterLink to="/invitations">Invitations</RouterLink>
        <RouterLink :to="`/u/${auth.currentUser.username}`">{{
          auth.currentUser.username
        }}</RouterLink>
        <RouterLink v-if="auth.currentUser.role === 'moderator'" to="/reports">Reports</RouterLink>
        <RouterLink v-if="auth.currentUser.role === 'moderator'" to="/addresses"
          >Addresses</RouterLink
        >
        <RouterLink v-if="auth.currentUser.role === 'moderator'" to="/section-settings"
          >Sections</RouterLink
        >
        <RouterLink to="/settings">Settings</RouterLink>
        <button type="button" @click="auth.doSignOut()">Sign out</button>
      </template>
      <template v-else>
        <RouterLink to="/sign-in">Sign in</RouterLink>
        <RouterLink to="/register">Register</RouterLink>
      </template>
    </nav>
  </header>

  <RouterView />
</template>

<style scoped>
header {
  padding: 1rem;
  border-bottom: 1px solid var(--color-border);
}

nav {
  display: flex;
  gap: 1rem;
  align-items: center;
}
</style>
