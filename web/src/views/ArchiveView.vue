<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getArchiveMonths, type ArchiveMonth } from '@/api/client'
import { monthLabel } from '@/lib/months'

const months = ref<ArchiveMonth[]>([])
const loadError = ref('')

async function load() {
  loadError.value = ''
  const result = await getArchiveMonths()
  if (result.ok) {
    months.value = result.value
  } else {
    loadError.value = result.error
  }
}

function subjectCount(topics: number): string {
  return topics === 1 ? '1 subject' : `${topics} subjects`
}

onMounted(load)
</script>

<template>
  <main>
    <h1>Archive</h1>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="entry in months" :key="`${entry.year}-${entry.month}`">
        <RouterLink :to="`/archive/${entry.year}/${entry.month}`">{{
          monthLabel(entry.year, entry.month)
        }}</RouterLink>
        <span>{{ subjectCount(entry.topics) }}</span>
      </li>
    </ul>
    <p v-if="months.length === 0 && !loadError">Nothing archived yet.</p>
  </main>
</template>
