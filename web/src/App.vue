<script setup lang="ts">
import { onMounted } from 'vue'
import { RouterLink, RouterView } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()

onMounted(() => {
  auth.checkSession()
})
</script>

<template>
  <header>
    <nav>
      <RouterLink to="/">Home</RouterLink>
      <template v-if="auth.currentUser">
        <RouterLink :to="`/u/${auth.currentUser.username}`">{{
          auth.currentUser.username
        }}</RouterLink>
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
