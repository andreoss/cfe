<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getTopicsByTag,
  getTag,
  describeTag,
  makeSynonym,
  followTag,
  unfollowTag,
  tagFeedUrl,
  type PageInfo,
  type Tag,
  type Topic,
} from '@/api/client'

const props = defineProps<{ tag: string }>()
const auth = useAuthStore()

const topics = ref<Topic[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')
const entry = ref<Tag | null>(null)
const descriptionDraft = ref('')
const meansDraft = ref('')
const followError = ref('')
const descriptionError = ref('')
const meansError = ref('')

function applyEntry(value: Tag) {
  entry.value = value
  descriptionDraft.value = value.description ?? ''
  meansDraft.value = value.means ?? ''
}

async function loadEntry() {
  const result = await getTag(props.tag)
  if (result.ok) applyEntry(result.value)
  else entry.value = null
}

async function toggleFollow() {
  followError.value = ''
  const following = entry.value?.following === true
  const result = following ? await unfollowTag(props.tag) : await followTag(props.tag)
  if (!result.ok) {
    followError.value = result.error
    return
  }
  const current = entry.value
  if (current === null) await loadEntry()
  else entry.value = { ...current, following: !following }
}

async function onSaveDescription() {
  descriptionError.value = ''
  const result = await describeTag(props.tag, descriptionDraft.value)
  if (result.ok) applyEntry(result.value)
  else descriptionError.value = result.error
}

async function onSetSynonym() {
  meansError.value = ''
  const result = await makeSynonym(props.tag, meansDraft.value)
  if (!result.ok) {
    meansError.value = result.error
    return
  }
  applyEntry(result.value)
  await goToPage(1)
}

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
  () => {
    void loadEntry()
    void goToPage(1)
  },
  { immediate: true },
)

watch(() => auth.currentUser, loadEntry)
</script>

<template>
  <main>
    <h1>#{{ tag }}</h1>
    <p v-if="entry && entry.description" class="dictionary">{{ entry.description }}</p>
    <p v-if="entry && entry.means" class="means">Means <RouterLink :to="`/tag/${entry.means}`">{{ entry.means }}</RouterLink></p>
    <p v-if="auth.currentUser" class="follow">
      <button type="button" @click="toggleFollow">
        {{ entry && entry.following ? 'Unfollow tag' : 'Follow tag' }}
      </button>
    </p>
    <p v-if="followError" role="alert">{{ followError }}</p>
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

    <section v-if="auth.currentUser?.role === 'moderator'" class="tag-admin">
      <form @submit.prevent="onSaveDescription">
        <label>
          Description
          <input v-model="descriptionDraft" name="tag-description" type="text" />
        </label>
        <p v-if="descriptionError" role="alert">{{ descriptionError }}</p>
        <button type="submit">Save description</button>
      </form>
      <form @submit.prevent="onSetSynonym">
        <label>
          Synonym for
          <input v-model="meansDraft" name="tag-means" type="text" />
        </label>
        <p v-if="meansError" role="alert">{{ meansError }}</p>
        <button type="submit">Set synonym</button>
      </form>
    </section>
  </main>
</template>

<style scoped>
main > ul {
  max-width: min(100%, 56rem);
}

.toolbar {
  font-size: var(--step-small);
}

.dictionary {
  color: var(--ink-soft);
}

.means {
  font-size: var(--step-small);
  color: var(--ink-soft);
}

.means a {
  font-weight: 600;
}

.tag-admin {
  display: flex;
  flex-direction: column;
  gap: var(--gap-4);
  margin-top: var(--gap-3);
  padding-top: var(--gap-4);
  border-top: 1px solid var(--edge-soft);
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
