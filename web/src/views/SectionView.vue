<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getTopics,
  createTopic,
  getGroups,
  commitTopic,
  uncommitTopic,
  sectionFeedUrl,
  type PageInfo,
  type Topic,
  type Group,
} from '@/api/client'

const props = defineProps<{ slug: string }>()
const auth = useAuthStore()

const topics = ref<Topic[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')
const creating = ref(false)
const titleDraft = ref('')
const bodyDraft = ref('')
const tagsDraft = ref('')
const groupDraft = ref('')
const groups = ref<Group[]>([])
const moderationError = ref('')
const formError = ref('')

function parseTags(raw: string): string[] {
  return raw
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t.length > 0)
}

async function loadGroups() {
  const result = await getGroups(props.slug)
  if (result.ok) groups.value = result.value
}

async function load() {
  loadError.value = ''
  const result = await getTopics(props.slug, currentPage.value)
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
  () => props.slug,
  () => {
    groups.value = []
    void loadGroups()
    void goToPage(1)
  },
  { immediate: true },
)

async function onCommit(id: string) {
  moderationError.value = ''
  const result = await commitTopic(id)
  if (result.ok) await load()
  else moderationError.value = result.error
}

async function onUncommit(id: string) {
  moderationError.value = ''
  const result = await uncommitTopic(id)
  if (result.ok) await load()
  else moderationError.value = result.error
}

async function onCreate() {
  formError.value = ''
  const result = await createTopic(
    props.slug,
    titleDraft.value,
    bodyDraft.value,
    parseTags(tagsDraft.value),
    groupDraft.value || undefined,
  )
  if (!result.ok) {
    formError.value = result.error
    return
  }
  titleDraft.value = ''
  bodyDraft.value = ''
  tagsDraft.value = ''
  groupDraft.value = ''
  creating.value = false
  await load()
}
</script>

<template>
  <main>
    <h1>{{ slug }}</h1>
    <p>
      <a :href="sectionFeedUrl(slug)">Atom feed</a>
      <RouterLink :to="`/s/${slug}/groups`">Groups</RouterLink>
    </p>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="topic in topics" :key="topic.id">
        <RouterLink :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
        <template v-if="topic.groupSlug">
          in <RouterLink :to="`/s/${topic.sectionSlug}/g/${topic.groupSlug}`">{{ topic.groupSlug }}</RouterLink>
        </template>
        by {{ topic.authorUsername }}
        <span v-if="topic.pending && auth.currentUser?.role === 'moderator'"> (pending)</span>
        <RouterLink v-for="tag in topic.tags" :key="tag" :to="`/tag/${tag}`">{{ tag }}</RouterLink>
        <button
          v-if="topic.pending && auth.currentUser?.role === 'moderator'"
          type="button"
          @click="onCommit(topic.id)"
        >
          Commit
        </button>
        <button
          v-if="!topic.pending && auth.currentUser?.role === 'moderator'"
          type="button"
          @click="onUncommit(topic.id)"
        >
          Uncommit
        </button>
      </li>
    </ul>
    <p v-if="moderationError" role="alert">{{ moderationError }}</p>
    <p v-if="topics.length === 0 && !loadError">No topics yet.</p>

    <nav v-if="page && page.totalPages > 1">
      <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
      <span>Page {{ page.number }} of {{ page.totalPages }}</span>
      <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
    </nav>

    <template v-if="auth.currentUser">
      <button v-if="!creating" type="button" @click="creating = true">New topic</button>
      <form v-else @submit.prevent="onCreate">
        <label>
          Title
          <input v-model="titleDraft" name="title" type="text" />
        </label>
        <label>
          Body
          <textarea v-model="bodyDraft" name="body" rows="6"></textarea>
        </label>
        <label>
          Tags (comma-separated)
          <input v-model="tagsDraft" name="tags" type="text" />
        </label>
        <label v-if="groups.length > 0">
          Group
          <select v-model="groupDraft" name="group">
            <option value="">— none —</option>
            <option v-for="g in groups" :key="g.id" :value="g.slug">{{ g.name }}</option>
          </select>
        </label>
        <p v-if="formError" role="alert">{{ formError }}</p>
        <button type="submit">Post</button>
        <button type="button" @click="creating = false">Cancel</button>
      </form>
    </template>
  </main>
</template>
