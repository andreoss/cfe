<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getSections, type Section } from '@/api/client'

const sections = ref<Section[]>([])
const loadError = ref('')

onMounted(async () => {
  const result = await getSections()
  if (result.ok) {
    sections.value = result.value
  } else {
    loadError.value = result.error
  }
})
</script>

<template>
  <main>
    <h1>Sections</h1>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="section in sections" :key="section.slug">
        <RouterLink :to="`/s/${section.slug}`">{{ section.title }}</RouterLink>
      </li>
    </ul>
  </main>
</template>
