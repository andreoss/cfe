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
        <RouterLink class="section-title" :to="`/s/${section.slug}`">{{ section.title }}</RouterLink>
      </li>
    </ul>
  </main>
</template>

<style scoped>
main > ul {
  max-width: min(100%, 44rem);
}

li {
  padding: 0;
}

li:hover {
  border-color: var(--edge);
}

.section-title {
  display: block;
  padding: var(--gap-3) var(--gap-4);
  font-size: var(--step-1);
  font-weight: 650;
  line-height: 1.3;
  color: var(--ink);
  text-decoration: none;
}

.section-title:hover {
  color: var(--accent);
  text-decoration: underline;
}
</style>
