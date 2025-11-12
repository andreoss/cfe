<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { search, type SearchHit } from '@/api/client'

const route = useRoute()
const router = useRouter()

const draft = ref('')
const hits = ref<SearchHit[]>([])
const searchError = ref('')
const searched = ref(false)

function currentQuery() {
  const q = route.query.q
  return typeof q === 'string' ? q : ''
}

async function load() {
  const q = currentQuery()
  draft.value = q
  searchError.value = ''
  if (q.length === 0) {
    hits.value = []
    searched.value = false
    return
  }
  const result = await search(q)
  searched.value = true
  if (result.ok) {
    hits.value = result.value
  } else {
    hits.value = []
    searchError.value = result.error
  }
}

watch(() => route.query.q, load, { immediate: true })

function onSubmit() {
  router.push({ name: 'search', query: { q: draft.value } })
}
</script>

<template>
  <main>
    <h1>Search</h1>
    <form @submit.prevent="onSubmit">
      <label>
        Query
        <input v-model="draft" name="query" type="text" />
      </label>
      <button type="submit">Search</button>
    </form>

    <p v-if="searchError" role="alert">{{ searchError }}</p>

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

    <p v-if="searched && hits.length === 0 && !searchError">Nothing found.</p>
  </main>
</template>
