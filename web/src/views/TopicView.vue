<script setup lang="ts">
import { ref, watch } from 'vue'
import { getTopic, type Topic } from '@/api/client'

const props = defineProps<{ id: string }>()

const topic = ref<Topic | null>(null)
const notFound = ref(false)

async function load() {
  notFound.value = false
  topic.value = null
  const result = await getTopic(props.id)
  if (result.ok) {
    topic.value = result.value
  } else {
    notFound.value = true
  }
}

watch(() => props.id, load, { immediate: true })
</script>

<template>
  <main>
    <p v-if="notFound">Topic not found.</p>
    <template v-else-if="topic">
      <h1>{{ topic.title }}</h1>
      <p>
        by <RouterLink :to="`/u/${topic.authorUsername}`">{{ topic.authorUsername }}</RouterLink>
        in <RouterLink :to="`/s/${topic.sectionSlug}`">{{ topic.sectionSlug }}</RouterLink>
      </p>
      <p v-if="topic.tags.length > 0">
        Tags:
        <RouterLink v-for="tag in topic.tags" :key="tag" :to="`/tag/${tag}`">{{ tag }}</RouterLink>
      </p>
      <p>{{ topic.body }}</p>
    </template>
  </main>
</template>
