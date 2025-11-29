<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getProfile,
  updateBio,
  uploadAvatar,
  deleteAvatar,
  getIgnoreState,
  ignoreUser,
  stopIgnoring,
  getRemark,
  setRemark,
  clearRemark,
  warnUser,
  banUser,
  liftBan,
  promoteUser,
  setUserRole,
  type Profile,
  type Role,
} from '@/api/client'
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

const ignoring = ref(false)
const ignoreReady = ref(false)
const warnOpen = ref(false)
const warnReason = ref('')
const banOpen = ref(false)
const banReason = ref('')
const banDays = ref('')
const moderationError = ref('')
const moderationStatus = ref('')
const roleDraft = ref<Role>('user')

const remarkDraft = ref('')
const remarkSaved = ref(false)
const remarkReady = ref(false)
const remarkStatus = ref('')
const remarkError = ref('')

const isOwnProfile = computed(() => auth.currentUser?.username === props.username)
const isOtherProfile = computed(() => auth.currentUser !== null && !isOwnProfile.value)
const isModerator = computed(() => auth.currentUser?.role === 'moderator')

async function load() {
  notFound.value = false
  profile.value = null
  ignoring.value = false
  ignoreReady.value = false
  warnOpen.value = false
  warnReason.value = ''
  banOpen.value = false
  banReason.value = ''
  banDays.value = ''
  moderationError.value = ''
  moderationStatus.value = ''
  roleDraft.value = 'user'
  const result = await getProfile(props.username)
  if (result.ok) {
    profile.value = result.value
    bioDraft.value = result.value.bio ?? ''
    roleDraft.value = result.value.role
  } else {
    notFound.value = true
    return
  }
}

async function loadIgnoreState() {
  if (!isOtherProfile.value) {
    ignoreReady.value = false
    return
  }
  const asked = props.username
  const state = await getIgnoreState(asked)
  if (asked !== props.username) return
  if (state.ok) {
    ignoring.value = state.value
  }
  ignoreReady.value = true
}

async function loadRemark() {
  remarkStatus.value = ''
  remarkError.value = ''
  remarkDraft.value = ''
  remarkSaved.value = false
  if (!isOtherProfile.value) {
    remarkReady.value = false
    return
  }
  const asked = props.username
  const result = await getRemark(asked)
  if (asked !== props.username) return
  if (result.ok) {
    remarkDraft.value = result.value ?? ''
    remarkSaved.value = result.value !== null
  } else {
    remarkError.value = result.error
  }
  remarkReady.value = true
}

watch(() => props.username, load, { immediate: true })
watch([() => props.username, () => auth.currentUser], loadIgnoreState, { immediate: true })
watch([() => props.username, () => auth.currentUser], loadRemark, { immediate: true })

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

async function onToggleIgnore() {
  moderationError.value = ''
  moderationStatus.value = ''
  const result = ignoring.value
    ? await stopIgnoring(props.username)
    : await ignoreUser(props.username)
  if (!result.ok) {
    moderationError.value = result.error
    return
  }
  ignoring.value = result.value
}

async function onSaveRemark() {
  remarkStatus.value = ''
  remarkError.value = ''
  const result = await setRemark(props.username, remarkDraft.value)
  if (!result.ok) {
    remarkError.value = result.error
    return
  }
  remarkSaved.value = true
  remarkStatus.value = 'Note saved.'
}

async function onClearRemark() {
  remarkStatus.value = ''
  remarkError.value = ''
  const result = await clearRemark(props.username)
  if (!result.ok) {
    remarkError.value = result.error
    return
  }
  remarkDraft.value = ''
  remarkSaved.value = false
  remarkStatus.value = 'Note cleared.'
}

async function onWarn() {
  moderationError.value = ''
  moderationStatus.value = ''
  const result = await warnUser(props.username, warnReason.value)
  if (!result.ok) {
    moderationError.value = result.error
    return
  }
  warnOpen.value = false
  warnReason.value = ''
  moderationStatus.value = 'Warning sent.'
}

async function onBan() {
  moderationError.value = ''
  moderationStatus.value = ''
  const days = banDays.value.trim()
  const result = await banUser(props.username, banReason.value, days ? Number(days) : null)
  if (!result.ok) {
    moderationError.value = result.error
    return
  }
  banOpen.value = false
  banReason.value = ''
  banDays.value = ''
  moderationStatus.value = 'User banned.'
}

async function onLiftBan() {
  moderationError.value = ''
  moderationStatus.value = ''
  const result = await liftBan(props.username)
  if (!result.ok) {
    moderationError.value = result.error
  }
}

async function onPromote() {
  moderationError.value = ''
  moderationStatus.value = ''
  const result = await promoteUser(props.username)
  if (!result.ok) {
    moderationError.value = result.error
    return
  }
  moderationStatus.value = 'User promoted.'
}

async function onSetRole() {
  moderationError.value = ''
  moderationStatus.value = ''
  const result = await setUserRole(props.username, roleDraft.value)
  if (!result.ok) {
    moderationError.value = result.error
    return
  }
  roleDraft.value = result.value.role
  if (profile.value !== null) {
    profile.value = { ...profile.value, role: result.value.role }
  }
  moderationStatus.value = 'Role updated.'
}
</script>

<template>
  <main>
    <p v-if="notFound">User not found.</p>
    <template v-else-if="profile">
      <UserAvatar :username="username" :size="96" :version="avatarVersion" />
      <h1>{{ profile.username }}</h1>
      <p data-test="score">Score: {{ profile.score }}</p>
      <template v-if="isOwnProfile">
        <input name="avatar-file" type="file" accept="image/*" @change="onFilePick" />
        <button type="button" @click="onUploadAvatar">Upload avatar</button>
        <button type="button" @click="onRemoveAvatar">Remove avatar</button>
        <p v-if="avatarError" role="alert">{{ avatarError }}</p>
      </template>
      <template v-if="isOtherProfile">
        <button v-if="ignoreReady" type="button" @click="onToggleIgnore">
          {{ ignoring ? 'Stop ignoring' : 'Ignore user' }}
        </button>
        <template v-if="remarkReady">
          <textarea v-model="remarkDraft" name="remark-text" rows="3"></textarea>
          <button type="button" @click="onSaveRemark">Save note</button>
          <button v-if="remarkSaved" type="button" @click="onClearRemark">Clear note</button>
          <p v-if="remarkStatus" role="status">{{ remarkStatus }}</p>
          <p v-if="remarkError" role="alert">{{ remarkError }}</p>
        </template>
        <template v-if="isModerator">
          <button v-if="!warnOpen" type="button" @click="warnOpen = true">Warn user</button>
          <form v-else @submit.prevent="onWarn">
            <label>
              Reason
              <input v-model="warnReason" name="warn-reason" type="text" />
            </label>
            <button type="submit">Send warning</button>
            <button type="button" @click="warnOpen = false">Cancel</button>
          </form>
          <button v-if="!banOpen" type="button" @click="banOpen = true">Ban user</button>
          <form v-else @submit.prevent="onBan">
            <label>
              Reason
              <input v-model="banReason" name="ban-reason" type="text" />
            </label>
            <label>
              Days
              <input v-model="banDays" name="ban-days" type="number" />
            </label>
            <button type="submit">Confirm ban</button>
            <button type="button" @click="banOpen = false">Cancel</button>
          </form>
          <button type="button" @click="onLiftBan">Lift ban</button>
          <button type="button" @click="onPromote">Promote to moderator</button>
          <label>
            Role
            <select v-model="roleDraft" name="user-role">
              <option value="user">Reader</option>
              <option value="corrector">Corrector</option>
              <option value="moderator">Moderator</option>
            </select>
          </label>
          <button type="button" @click="onSetRole">Set role</button>
        </template>
        <p v-if="moderationStatus" role="status">{{ moderationStatus }}</p>
        <p v-if="moderationError" role="alert">{{ moderationError }}</p>
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
