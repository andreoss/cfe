<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { getProfile, updateBio, uploadAvatar, deleteAvatar, type Profile } from '@/api/client'
import UserAvatar from '@/components/UserAvatar.vue'

const props = defineProps<{ username: string }>()
const auth = useAuthStore()

const profile = ref<Profile | null>(null)
const notFound = ref(false)
const editing = ref(false)
const bioDraft = ref('')
const formError = ref('')
const avatarError = ref('')
const avatarVersion = ref(0)
const avatarFile = ref<File | null>(null)

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

function onFilePick(event: Event) {
  const input = event.target as HTMLInputElement
  avatarFile.value = input.files?.[0] ?? null
}

function readBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const text = typeof reader.result === 'string' ? reader.result : ''
      const comma = text.indexOf(',')
      resolve(comma >= 0 ? text.slice(comma + 1) : text)
    }
    reader.onerror = () => reject(new Error('file could not be read'))
    reader.readAsDataURL(file)
  })
}

async function onUploadAvatar() {
  avatarError.value = ''
  const file = avatarFile.value
  if (file === null) return
  const base64 = await readBase64(file)
  const result = await uploadAvatar(base64)
  if (!result.ok) {
    avatarError.value = result.error
    return
  }
  avatarVersion.value += 1
}

async function onRemoveAvatar() {
  avatarError.value = ''
  const result = await deleteAvatar()
  if (!result.ok) {
    avatarError.value = result.error
    return
  }
  avatarVersion.value += 1
}
</script>

<template>
  <main>
    <p v-if="notFound">User not found.</p>
    <template v-else-if="profile">
      <UserAvatar :username="username" :size="96" :version="avatarVersion" />
      <h1>{{ profile.username }}</h1>
      <template v-if="isOwnProfile">
        <input name="avatar-file" type="file" accept="image/*" @change="onFilePick" />
        <button type="button" @click="onUploadAvatar">Upload avatar</button>
        <button type="button" @click="onRemoveAvatar">Remove avatar</button>
        <p v-if="avatarError" role="alert">{{ avatarError }}</p>
      </template>
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
