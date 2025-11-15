<script setup lang="ts">
import { ref, watch } from 'vue'
import { getTopicsByTag, tagFeedUrl, type Topic } from '@/api/client'

const props = defineProps<{ tag: string }>()

const topics = ref<Topic[]>([])
const loadError = ref('')

async function load() {
  loadError.value = ''
  const result = await getTopicsByTag(props.tag)
  if (result.ok) {
    topics.value = result.value
  } else {
    loadError.value = result.error
  }
}

watch(() => props.tag, load, { immediate: true })
</script>

<template>
  <main>
    <h1>#{{ tag }}</h1>
    <p><a :href="tagFeedUrl(tag)">Atom feed</a></p>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="topic in topics" :key="topic.id">
        <RouterLink :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
        in <RouterLink :to="`/s/${topic.sectionSlug}`">{{ topic.sectionSlug }}</RouterLink>
      </li>
    </ul>
    <p v-if="topics.length === 0 && !loadError">No topics with this tag yet.</p>
  </main>
</template>
