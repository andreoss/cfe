<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { getFollowedTags } from '@/api/client'

const auth = useAuthStore()
const tags = ref<string[]>([])
const loadError = ref('')

async function load() {
  loadError.value = ''
  if (auth.currentUser === null) {
    tags.value = []
    return
  }
  const result = await getFollowedTags()
  if (result.ok) tags.value = result.value
  else loadError.value = result.error
}

watch(() => auth.currentUser, load, { immediate: true })
</script>

<template>
  <main>
    <p v-if="!auth.currentUser">Sign in to see the tags you follow.</p>
    <template v-else>
      <h1>Followed tags</h1>
      <p v-if="loadError" role="alert">{{ loadError }}</p>
      <ul>
        <li v-for="slug in tags" :key="slug">
          <RouterLink :to="`/tag/${slug}`">#{{ slug }}</RouterLink>
        </li>
      </ul>
      <p v-if="tags.length === 0 && !loadError">You follow no tags.</p>
    </template>
  </main>
</template>

<style scoped>
li a {
  font-size: var(--step-1);
}
</style>
