<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  search,
  toSearchOrder,
  toSearchScope,
  type SearchHit,
  type SearchOrder,
  type SearchScope,
} from '@/api/client'

const route = useRoute()
const router = useRouter()

const draft = ref('')
const scope = ref<SearchScope>('everything')
const order = ref<SearchOrder>('relevance')
const hits = ref<SearchHit[]>([])
const searchError = ref('')
const searched = ref(false)

const scopes: { value: SearchScope; label: string }[] = [
  { value: 'everything', label: 'Everything' },
  { value: 'topics', label: 'Topics' },
  { value: 'comments', label: 'Comments' },
]

const orders: { value: SearchOrder; label: string }[] = [
  { value: 'relevance', label: 'Best match' },
  { value: 'newest', label: 'Newest first' },
  { value: 'oldest', label: 'Oldest first' },
]

function currentQuery() {
  const q = route.query.q
  return typeof q === 'string' ? q : ''
}

async function load() {
  const q = currentQuery()
  draft.value = q
  scope.value = toSearchScope(route.query.scope)
  order.value = toSearchOrder(route.query.order)
  searchError.value = ''
  if (q.length === 0) {
    hits.value = []
    searched.value = false
    return
  }
  const result = await search(q, scope.value, order.value)
  searched.value = true
  if (result.ok) {
    hits.value = result.value
  } else {
    hits.value = []
    searchError.value = result.error
  }
}

watch(() => [route.query.q, route.query.scope, route.query.order], load, { immediate: true })

function ask() {
  router.push({
    name: 'search',
    query: { q: draft.value, scope: scope.value, order: order.value },
  })
}

function onSubmit() {
  ask()
}

function onNarrow() {
  if (currentQuery().length > 0) ask()
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
      <label>
        Look in
        <select v-model="scope" name="scope" @change="onNarrow">
          <option v-for="choice in scopes" :key="choice.value" :value="choice.value">
            {{ choice.label }}
          </option>
        </select>
      </label>
      <label>
        Order
        <select v-model="order" name="order" @change="onNarrow">
          <option v-for="choice in orders" :key="choice.value" :value="choice.value">
            {{ choice.label }}
          </option>
        </select>
      </label>
      <button type="submit">Search</button>
    </form>

    <p v-if="searchError" role="alert">{{ searchError }}</p>

    <ul>
      <li
        v-for="hit in hits"
        :key="hit.kind === 'topic' ? hit.topic.id : hit.comment.id"
        :class="`hit hit-${hit.kind}`"
      >
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

<style scoped>
form {
  flex-direction: row;
  align-items: flex-end;
  gap: var(--gap-3);
  flex-wrap: wrap;
}

form label:first-of-type {
  flex: 1 1 12rem;
}

li {
  display: flex;
  flex-direction: column;
  gap: var(--gap-1);
}

.excerpt {
  color: var(--ink-soft);
  font-size: var(--step-small);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
