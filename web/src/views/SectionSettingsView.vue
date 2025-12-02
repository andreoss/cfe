<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getSections,
  createSection,
  renameSection,
  setSectionScore,
  renameGroup,
  TOPICS_SCORES,
  type Section,
} from '@/api/client'

const auth = useAuthStore()

const sections = ref<Section[]>([])
const loadError = ref('')
const actionError = ref('')
const saved = ref(false)

const isModerator = computed(() => auth.currentUser?.role === 'moderator')
const denied = computed(() => auth.checked && !isModerator.value)

const sectionSlugDraft = ref('')
const sectionTitleDraft = ref('')
const renameSlugDraft = ref('')
const renameTitleDraft = ref('')
const scoreSlugDraft = ref('')
const scoreDraft = ref<string>(TOPICS_SCORES[0])
const groupSectionDraft = ref('')
const groupSlugDraft = ref('')
const groupTitleDraft = ref('')

async function load() {
  if (!isModerator.value) {
    sections.value = []
    loadError.value = ''
    return
  }
  loadError.value = ''
  const result = await getSections()
  if (result.ok) {
    sections.value = result.value
  } else {
    loadError.value = result.error
  }
}

function begin() {
  saved.value = false
  actionError.value = ''
}

async function onCreateSection() {
  begin()
  const result = await createSection(sectionSlugDraft.value, sectionTitleDraft.value)
  if (!result.ok) {
    actionError.value = result.error
    return
  }
  sectionSlugDraft.value = ''
  sectionTitleDraft.value = ''
  saved.value = true
  await load()
}

async function onRenameSection() {
  begin()
  const result = await renameSection(renameSlugDraft.value, renameTitleDraft.value)
  if (!result.ok) {
    actionError.value = result.error
    return
  }
  renameSlugDraft.value = ''
  renameTitleDraft.value = ''
  saved.value = true
  await load()
}

async function onSetScore() {
  begin()
  const result = await setSectionScore(scoreSlugDraft.value, scoreDraft.value)
  if (!result.ok) {
    actionError.value = result.error
    return
  }
  scoreSlugDraft.value = ''
  saved.value = true
  await load()
}

async function onRenameGroup() {
  begin()
  const result = await renameGroup(
    groupSectionDraft.value,
    groupSlugDraft.value,
    groupTitleDraft.value,
  )
  if (!result.ok) {
    actionError.value = result.error
    return
  }
  groupSectionDraft.value = ''
  groupSlugDraft.value = ''
  groupTitleDraft.value = ''
  saved.value = true
}

watch(() => auth.currentUser, load, { immediate: true })
</script>

<template>
  <main>
    <h1>Section settings</h1>
    <p v-if="denied">Only a moderator may configure sections.</p>
    <template v-else-if="isModerator">
      <p v-if="loadError" role="alert">{{ loadError }}</p>
      <p v-if="saved" role="status">Saved.</p>
      <p v-if="actionError" role="alert">{{ actionError }}</p>

      <ul>
        <li v-for="section in sections" :key="section.slug">
          <span>{{ section.slug }}</span>
          <span>{{ section.title }}</span>
        </li>
      </ul>

      <div class="forms">
        <form @submit.prevent="onCreateSection">
          <label>
            Slug
            <input v-model="sectionSlugDraft" name="section-slug" type="text" />
          </label>
          <label>
            Title
            <input v-model="sectionTitleDraft" name="section-title" type="text" />
          </label>
          <button type="submit">Add section</button>
        </form>

        <form @submit.prevent="onRenameSection">
          <label>
            Section
            <input v-model="renameSlugDraft" name="rename-slug" type="text" />
          </label>
          <label>
            Title
            <input v-model="renameTitleDraft" name="rename-title" type="text" />
          </label>
          <button type="submit">Rename section</button>
        </form>

        <form @submit.prevent="onSetScore">
          <label>
            Section
            <input v-model="scoreSlugDraft" name="score-slug" type="text" />
          </label>
          <label>
            Standing
            <select v-model="scoreDraft" name="topics-score">
              <option v-for="score in TOPICS_SCORES" :key="score" :value="score">{{ score }}</option>
            </select>
          </label>
          <button type="submit">Set standing</button>
        </form>

        <form @submit.prevent="onRenameGroup">
          <label>
            Section
            <input v-model="groupSectionDraft" name="group-section" type="text" />
          </label>
          <label>
            Group
            <input v-model="groupSlugDraft" name="group-slug" type="text" />
          </label>
          <label>
            Title
            <input v-model="groupTitleDraft" name="group-title" type="text" />
          </label>
          <button type="submit">Rename group</button>
        </form>
      </div>
    </template>
  </main>
</template>

<style scoped>
main > p:not([role]) {
  color: var(--ink-soft);
}

p[role='status'] {
  padding: var(--gap-2) var(--gap-3);
  border-radius: var(--round);
  background: var(--good-soft);
  color: var(--good);
  font-size: var(--step-small);
  font-weight: 550;
}

main > ul {
  gap: var(--gap-1);
}

main > ul > li {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--gap-1) var(--gap-3);
  padding: var(--gap-2) var(--gap-3);
  border-radius: var(--round);
  box-shadow: none;
}

main > ul > li > span:first-child {
  min-width: 10ch;
  font-family: ui-monospace, 'SFMono-Regular', 'Cascadia Mono', Menlo, monospace;
  font-size: var(--step-small);
  color: var(--ink-faint);
}

main > ul > li > span:last-child {
  font-weight: 550;
}

.forms {
  max-width: none;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
  gap: var(--gap-4);
  align-items: start;
}

.forms > form {
  height: 100%;
  gap: var(--gap-3);
  border: 1px solid var(--edge);
  box-shadow: var(--shadow);
}

.forms > form > button {
  margin-top: auto;
}

@media (max-width: 40rem) {
  .forms {
    grid-template-columns: 1fr;
    gap: var(--gap-3);
  }
}
</style>
