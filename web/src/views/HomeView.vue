<script setup lang="ts">
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
</script>

<template>
  <main>
    <p v-if="auth.currentUser">Signed in as {{ auth.currentUser.username }}.</p>
    <p v-else>Not signed in.</p>
  </main>
</template>
