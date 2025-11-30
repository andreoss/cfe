<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { getArchiveMonth, type PageInfo, type Topic } from '@/api/client'
import { monthLabel } from '@/lib/months'

const props = defineProps<{ year: string; month: string }>()

const topics = ref<Topic[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')

const heading = computed(() => monthLabel(Number(props.year), Number(props.month)))

async function load() {
  loadError.value = ''
  const result = await getArchiveMonth(
    Number(props.year),
    Number(props.month),
    currentPage.value,
  )
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

watch(
  () => [props.year, props.month],
  () => goToPage(1),
  { immediate: true },
)
</script>

<template>
  <main>
    <h1>{{ heading }}</h1>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="topic in topics" :key="topic.id">
        <RouterLink :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
        in <RouterLink :to="`/s/${topic.sectionSlug}`">{{ topic.sectionSlug }}</RouterLink>
      </li>
    </ul>
    <p v-if="topics.length === 0 && !loadError">Nothing posted in this month.</p>

    <nav v-if="page && page.totalPages > 1">
      <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
      <span>Page {{ page.number }} of {{ page.totalPages }}</span>
      <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
    </nav>
  </main>
</template>
