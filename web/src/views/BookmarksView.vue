<script setup lang="ts">
import { ref } from 'vue'
import { getBookmarks, type Topic } from '@/api/client'

const topics = ref<Topic[]>([])
const loadError = ref('')

async function load() {
  loadError.value = ''
  const result = await getBookmarks()
  if (result.ok) {
    topics.value = result.value
  } else {
    loadError.value = result.error
  }
}

load()
</script>

<template>
  <main>
    <h1>Bookmarks</h1>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="topic in topics" :key="topic.id">
        <RouterLink :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
      </li>
    </ul>
    <p v-if="topics.length === 0 && !loadError">No saved topics.</p>
  </main>
</template>
