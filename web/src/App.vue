<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
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

const POLL_MS = Number(import.meta.env.VITE_POLL_MS ?? 15000)
let watcher: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  auth.checkSession()
  if (POLL_MS > 0) {
    watcher = setInterval(() => {
      if (auth.currentUser && !document.hidden) void loadUnreadCount()
    }, POLL_MS)
  }
})

onUnmounted(() => {
  if (watcher !== null) clearInterval(watcher)
})
</script>

<template>
  <a class="skip" href="#content">Skip to content</a>
  <header>
    <nav>
      <RouterLink to="/">Home</RouterLink>
      <RouterLink to="/search">Search</RouterLink>
      <RouterLink to="/activity">Activity</RouterLink>
      <RouterLink to="/archive">Archive</RouterLink>
      <span class="spacer"></span>
      <template v-if="auth.currentUser">
        <RouterLink to="/notifications">{{
          unreadCount > 0 ? `Notifications (${unreadCount})` : 'Notifications'
        }}</RouterLink>
        <RouterLink to="/bookmarks">Saved</RouterLink>
        <RouterLink to="/watched">Watched</RouterLink>
        <RouterLink to="/followed-tags">Followed tags</RouterLink>
        <RouterLink to="/notes">Notes</RouterLink>
        <RouterLink to="/invitations">Invitations</RouterLink>
        <RouterLink :to="`/u/${auth.currentUser.username}`">{{
          auth.currentUser.username
        }}</RouterLink>
        <RouterLink v-if="auth.currentUser.role === 'moderator'" to="/operator"
          >Operator</RouterLink
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

  <div id="content">
    <RouterView />
  </div>
</template>

<style scoped>
.skip {
  position: absolute;
  left: -9999px;
  top: 0;
  padding: var(--gap-2) var(--gap-3);
  background: var(--accent);
  color: var(--accent-ink);
  border-radius: var(--round);
  z-index: 20;
}

.skip:focus {
  left: var(--gap-3);
  top: var(--gap-2);
}

nav :deep(a),
nav button {
  flex: 0 0 auto;
}

nav .spacer {
  flex: 1 1 auto;
}

nav button {
  font-size: var(--step-small);
}

@media (max-width: 40rem) {
  nav .spacer {
    flex-basis: 100%;
    height: 0;
  }
}
</style>
