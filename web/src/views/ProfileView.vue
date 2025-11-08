<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { getProfile, updateBio, type Profile } from '@/api/client'

const props = defineProps<{ username: string }>()
const auth = useAuthStore()

const profile = ref<Profile | null>(null)
const notFound = ref(false)
const editing = ref(false)
const bioDraft = ref('')
const formError = ref('')

const isOwnProfile = computed(() => auth.currentUser?.username === props.username)

async function load() {
  notFound.value = false
  profile.value = null
  const result = await getProfile(props.username)
  if (result.ok) {
    profile.value = result.value
    bioDraft.value = result.value.bio ?? ''
  } else {
    notFound.value = true
  }
}

watch(() => props.username, load, { immediate: true })

async function onSave() {
  formError.value = ''
  const result = await updateBio(bioDraft.value)
  if (!result.ok) {
    formError.value = result.error
    return
  }
  profile.value = result.value
  editing.value = false
}
</script>

<template>
  <main>
    <p v-if="notFound">User not found.</p>
    <template v-else-if="profile">
      <h1>{{ profile.username }}</h1>
      <template v-if="editing">
        <textarea v-model="bioDraft" name="bio" rows="4"></textarea>
        <p v-if="formError" role="alert">{{ formError }}</p>
        <button type="button" @click="onSave">Save</button>
        <button type="button" @click="editing = false">Cancel</button>
      </template>
      <template v-else>
        <p>{{ profile.bio ?? 'No bio yet.' }}</p>
        <button v-if="isOwnProfile" type="button" @click="editing = true">Edit bio</button>
      </template>
    </template>
  </main>
</template>
