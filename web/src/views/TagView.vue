<script setup lang="ts">
import { ref, watch } from 'vue'
import { getTopicsByTag, tagFeedUrl, type PageInfo, type Topic } from '@/api/client'

const props = defineProps<{ tag: string }>()

const topics = ref<Topic[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')

async function load() {
  loadError.value = ''
  const result = await getTopicsByTag(props.tag, currentPage.value)
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
  () => props.tag,
  () => goToPage(1),
  { immediate: true },
)
</script>

<template>
  <main>
    <h1>#{{ tag }}</h1>
    <p class="toolbar"><a :href="tagFeedUrl(tag)">Atom feed</a></p>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="topic in topics" :key="topic.id">
        <RouterLink class="topic-title" :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
        in <RouterLink :to="`/s/${topic.sectionSlug}`">{{ topic.sectionSlug }}</RouterLink>
      </li>
    </ul>
    <p v-if="topics.length === 0 && !loadError">No topics with this tag yet.</p>

    <nav v-if="page && page.totalPages > 1">
      <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
      <span>Page {{ page.number }} of {{ page.totalPages }}</span>
      <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
    </nav>
  </main>
</template>

<style scoped>
main > ul {
  max-width: min(100%, 56rem);
}

.toolbar {
  font-size: var(--step-small);
}

li {
  font-size: var(--step-small);
  line-height: 1.7;
  color: var(--ink-soft);
}

li:hover {
  border-color: var(--edge);
}

.topic-title {
  display: block;
  margin-bottom: var(--gap-1);
  font-size: var(--step-1);
  font-weight: 650;
  line-height: 1.3;
  color: var(--ink);
  text-decoration: none;
}

.topic-title:hover {
  color: var(--accent);
  text-decoration: underline;
}

li a:not(.topic-title) {
  font-weight: 550;
  text-decoration: none;
}

li a:not(.topic-title):hover {
  text-decoration: underline;
}

main > nav {
  align-self: flex-start;
  display: flex;
  align-items: stretch;
  border: 1px solid var(--edge);
  border-radius: var(--round);
  background: var(--ground);
}

main > nav button {
  border: 0;
  border-radius: 0;
  background: transparent;
  padding: var(--gap-2) var(--gap-4);
}

main > nav span {
  display: flex;
  align-items: center;
  padding: 0 var(--gap-4);
  border-left: 1px solid var(--edge-soft);
  border-right: 1px solid var(--edge-soft);
  font-size: var(--step-small);
  color: var(--ink-soft);
  white-space: nowrap;
}

@media (max-width: 40rem) {
  main > nav {
    align-self: stretch;
  }

  main > nav span {
    flex: 1;
    justify-content: center;
  }
}
</style>
