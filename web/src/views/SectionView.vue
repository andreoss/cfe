<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { getTopics, createTopic, type Topic } from '@/api/client'

const props = defineProps<{ slug: string }>()
const auth = useAuthStore()

const topics = ref<Topic[]>([])
const loadError = ref('')
const creating = ref(false)
const titleDraft = ref('')
const bodyDraft = ref('')
const tagsDraft = ref('')
const formError = ref('')

function parseTags(raw: string): string[] {
  return raw
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t.length > 0)
}

async function load() {
  loadError.value = ''
  const result = await getTopics(props.slug)
  if (result.ok) {
    topics.value = result.value
  } else {
    loadError.value = result.error
  }
}

watch(() => props.slug, load, { immediate: true })

async function onCreate() {
  formError.value = ''
  const result = await createTopic(
    props.slug,
    titleDraft.value,
    bodyDraft.value,
    parseTags(tagsDraft.value),
  )
  if (!result.ok) {
    formError.value = result.error
    return
  }
  titleDraft.value = ''
  bodyDraft.value = ''
  tagsDraft.value = ''
  creating.value = false
  await load()
}
</script>

<template>
  <main>
    <h1>{{ slug }}</h1>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="topic in topics" :key="topic.id">
        <RouterLink :to="`/t/${topic.id}`">{{ topic.title }}</RouterLink>
        by {{ topic.authorUsername }}
        <RouterLink v-for="tag in topic.tags" :key="tag" :to="`/tag/${tag}`">{{ tag }}</RouterLink>
      </li>
    </ul>
    <p v-if="topics.length === 0 && !loadError">No topics yet.</p>

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
        <p v-if="formError" role="alert">{{ formError }}</p>
        <button type="submit">Post</button>
        <button type="button" @click="creating = false">Cancel</button>
      </form>
    </template>
  </main>
</template>
