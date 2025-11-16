<script setup lang="ts">
import { ref } from 'vue'
import { getActivity, type SearchHit } from '@/api/client'

const hits = ref<SearchHit[]>([])
const loadError = ref('')

async function load() {
  loadError.value = ''
  const result = await getActivity()
  if (result.ok) {
    hits.value = result.value
  } else {
    hits.value = []
    loadError.value = result.error
  }
}

load()
</script>

<template>
  <main>
    <h1>Activity</h1>

    <p v-if="loadError" role="alert">{{ loadError }}</p>

    <ul>
      <li v-for="hit in hits" :key="hit.kind === 'topic' ? hit.topic.id : hit.comment.id">
        <template v-if="hit.kind === 'topic'">
          <RouterLink :to="`/t/${hit.topic.id}`">{{ hit.topic.title }}</RouterLink>
          in <RouterLink :to="`/s/${hit.topic.sectionSlug}`">{{ hit.topic.sectionSlug }}</RouterLink>
        </template>
        <template v-else>
          <RouterLink :to="`/t/${hit.comment.topicId}`">
            Comment by {{ hit.comment.authorUsername }}
          </RouterLink>
          <span class="excerpt">{{ hit.comment.body }}</span>
        </template>
      </li>
    </ul>

    <p v-if="hits.length === 0 && !loadError">No activity yet.</p>
  </main>
</template>
