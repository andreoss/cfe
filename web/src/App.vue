<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
</script>

<template>
  <header>
    <nav>
      <RouterLink to="/">Home</RouterLink>
      <span v-if="auth.currentUser">{{ auth.currentUser.username }}</span>
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
}
</style>
