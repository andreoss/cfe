<script setup lang="ts">
import { ref, watch } from 'vue'
import { getGroups, getTopics, type Group, type Topic, type PageInfo } from '@/api/client'

const props = defineProps<{ slug: string; group: string }>()

const groupInfo = ref<Group | null>(null)
const topics = ref<Topic[]>([])
const page = ref<PageInfo | null>(null)
const loadError = ref('')

async function load() {
  loadError.value = ''
  const groupsResult = await getGroups(props.slug)
  if (groupsResult.ok) {
    groupInfo.value = groupsResult.value.find((g) => g.slug === props.group) ?? null
  }
  const topicsResult = await getTopics(props.slug)
  if (topicsResult.ok) {
    topics.value = topicsResult.value.items.filter((t) => t.groupSlug === props.group)
    page.value = topicsResult.value.page
  } else {
    loadError.value = topicsResult.error
  }
}

watch(() => [props.slug, props.group], load, { immediate: true })
</script>

<template>
  <main>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <h1 v-if="groupInfo">{{ groupInfo.name }}</h1>
    <p v-else-if="!loadError">Group "{{ group }}" not found.</p>
    <ul>
      <li v-for="topic in topics" :key="topic.id">
        <RouterLink :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
        by {{ topic.authorUsername }}
      </li>
    </ul>
    <p v-if="topics.length === 0 && !loadError">No topics in this group yet.</p>
  </main>
</template>
