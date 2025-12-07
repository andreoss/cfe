<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { getWatched, type PageInfo, type Topic } from '@/api/client'

const auth = useAuthStore()
const topics = ref<Topic[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')

async function load() {
  if (auth.currentUser === null) {
    topics.value = []
    page.value = null
    loadError.value = ''
    return
  }
  loadError.value = ''
  const result = await getWatched(currentPage.value)
  if (result.ok) {
    topics.value = result.value.items
    page.value = result.value.page
  } else {
    loadError.value = result.error
  }
}

async function goToPage(target: number) {
  currentPage.value = target
  await load()
}

function previousPage() {
  if (page.value) void goToPage(page.value.number - 1)
}

function nextPage() {
  if (page.value) void goToPage(page.value.number + 1)
}

watch(() => auth.currentUser, load, { immediate: true })
</script>

<template>
  <main>
    <p v-if="auth.checked && !auth.currentUser">Sign in to see what you are watching.</p>
    <template v-else>
      <h1>Watched</h1>
      <p v-if="loadError" role="alert">{{ loadError }}</p>
      <ul>
        <li v-for="topic in topics" :key="topic.id">
          <RouterLink :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
        </li>
      </ul>
      <p v-if="topics.length === 0 && !loadError">You are not watching anything.</p>

      <nav v-if="page && page.totalPages > 1">
        <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
        <span>Page {{ page.number }} of {{ page.totalPages }}</span>
        <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
      </nav>
    </template>
  </main>
</template>
