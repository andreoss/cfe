<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getTopics,
  createTopic,
  getGroups,
  getSections,
  commitTopic,
  uncommitTopic,
  sectionFeedUrl,
  type PageInfo,
  type Topic,
  type Group,
  type Section,
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
const asDraft = ref(false)
const groups = ref<Group[]>([])
const section = ref<Section | null>(null)
const moderationError = ref('')
const formError = ref('')
const challengeNeeded = ref(false)
const challengeAnswer = ref('')

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

async function loadSection() {
  const result = await getSections()
  if (result.ok) section.value = result.value.find((s) => s.slug === props.slug) ?? null
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
    section.value = null
    void loadGroups()
    void loadSection()
    void goToPage(1)
  },
  { immediate: true },
)

watch(() => auth.currentUser, loadSection)

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
    challengeAnswer.value,
    asDraft.value,
  )
  if (!result.ok) {
    if (result.status === 428 && !challengeNeeded.value) {
      challengeNeeded.value = true
      challengeAnswer.value = ''
      return
    }
    formError.value = result.error
    return
  }
  titleDraft.value = ''
  bodyDraft.value = ''
  tagsDraft.value = ''
  groupDraft.value = ''
  asDraft.value = false
  challengeNeeded.value = false
  challengeAnswer.value = ''
  creating.value = false
  await load()
}
</script>

<template>
  <main>
    <h1>{{ slug }}</h1>
    <p class="toolbar">
      <a :href="sectionFeedUrl(slug)">Atom feed</a>
      <RouterLink :to="`/s/${slug}/groups`">Groups</RouterLink>
    </p>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="topic in topics" :key="topic.id" :class="{ pinned: topic.sticky }">
        <RouterLink class="topic-title" :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
        <template v-if="topic.groupSlug">
          in <RouterLink :to="`/s/${topic.sectionSlug}/g/${topic.groupSlug}`">{{ topic.groupSlug }}</RouterLink>
        </template>
        by {{ topic.authorUsername }}
        <span v-if="topic.draft" class="flag flag-draft">Draft</span>
        <span v-if="topic.sticky" class="flag flag-sticky">Sticky</span>
        <span v-if="topic.pending && auth.currentUser?.role === 'moderator'" class="pending-note"> (pending)</span>
        <RouterLink v-for="tag in topic.tags" :key="tag" class="tag" :to="`/tag/${tag}`">{{ tag }}</RouterLink>
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

    <template v-if="section?.mayPost">
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
        <label>
          <input v-model="asDraft" name="topic-draft" type="checkbox" />
          Save as draft
        </label>
        <template v-if="challengeNeeded">
          <p role="status">Answer the challenge to continue.</p>
          <label>
            Challenge
            <input v-model="challengeAnswer" name="topic-challenge-answer" type="text" />
          </label>
        </template>
        <p v-if="formError" role="alert">{{ formError }}</p>
        <button type="submit">Post</button>
        <button type="button" @click="creating = false">Cancel</button>
      </form>
    </template>
  </main>
</template>

<style scoped>
main > ul {
  max-width: min(100%, 56rem);
  gap: var(--gap-2);
}

.toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gap-1) var(--gap-4);
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

li.pinned {
  box-shadow: inset 3px 0 0 var(--accent), var(--shadow);
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

.flag,
.tag {
  display: inline-block;
  padding: 0.05rem var(--gap-2);
  border-radius: 999px;
  font-size: var(--step-tiny);
  font-weight: 600;
  line-height: 1.6;
  white-space: nowrap;
}

.flag-draft {
  background: var(--warn-soft);
  color: var(--ink-soft);
}

.flag-sticky {
  background: var(--accent-soft);
  color: var(--accent);
}

.pending-note {
  color: var(--ink-faint);
}

li a.tag {
  background: var(--ground-sunk);
  color: var(--ink-soft);
  font-weight: 550;
}

li a.tag:hover {
  background: var(--accent-soft);
  color: var(--accent);
  text-decoration: none;
}

li button {
  margin-top: var(--gap-2);
  margin-right: var(--gap-1);
  font-size: var(--step-tiny);
  padding: 0.2rem 0.6rem;
}

main > nav {
  align-self: flex-start;
  display: flex;
  align-items: stretch;
  border: 1px solid var(--edge);
  border-radius: var(--round);
  background: var(--ground);
  overflow: visible;
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

main > button {
  align-self: flex-start;
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
