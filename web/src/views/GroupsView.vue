<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getGroups,
  createGroup,
  type Group,
} from '@/api/client'

const props = defineProps<{ slug: string }>()
const auth = useAuthStore()

const groups = ref<Group[]>([])
const loadError = ref('')
const creating = ref(false)
const nameDraft = ref('')
const slugDraft = ref('')
const formError = ref('')

async function load() {
  loadError.value = ''
  const result = await getGroups(props.slug)
  if (result.ok) groups.value = result.value
  else loadError.value = result.error
}

watch(() => props.slug, load, { immediate: true })

async function onCreate() {
  formError.value = ''
  const result = await createGroup(props.slug, nameDraft.value, slugDraft.value)
  if (!result.ok) {
    formError.value = result.error
    return
  }
  nameDraft.value = ''
  slugDraft.value = ''
  creating.value = false
  await load()
}
</script>

<template>
  <main>
    <h1>Groups in {{ slug }}</h1>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="g in groups" :key="g.id">
        <RouterLink :to="`/s/${g.sectionSlug}/g/${g.slug}`">{{ g.name }}</RouterLink>
      </li>
    </ul>
    <p v-if="groups.length === 0 && !loadError">No groups yet.</p>

    <template v-if="auth.currentUser?.role === 'moderator'">
      <button v-if="!creating" type="button" @click="creating = true">New group</button>
      <form v-else @submit.prevent="onCreate">
        <label>
          Name
          <input v-model="nameDraft" name="name" type="text" />
        </label>
        <label>
          Slug
          <input v-model="slugDraft" name="slug" type="text" />
        </label>
        <p v-if="formError" role="alert">{{ formError }}</p>
        <button type="submit">Create</button>
        <button type="button" @click="creating = false">Cancel</button>
      </form>
    </template>
  </main>
</template>
