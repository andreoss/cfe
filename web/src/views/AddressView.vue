<script setup lang="ts">
import { computed, ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  listAddressPosts,
  removeAddressPosts,
  type AddressPost,
  type PageInfo,
} from '@/api/client'

const auth = useAuthStore()

const addrDraft = ref('')
const investigated = ref('')
const posts = ref<AddressPost[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const listed = ref(false)
const loadError = ref('')

const hoursDraft = ref(24)
const reasonDraft = ref('')
const removeError = ref('')
const removed = ref<number | null>(null)

const isModerator = computed(() => auth.currentUser?.role === 'moderator')
const denied = computed(() => auth.checked && !isModerator.value)

async function load() {
  loadError.value = ''
  const result = await listAddressPosts(investigated.value, currentPage.value)
  if (!result.ok) {
    loadError.value = result.error
    return
  }
  posts.value = result.value.items
  page.value = result.value.page
  listed.value = true
}

async function onInvestigate() {
  investigated.value = addrDraft.value
  currentPage.value = 1
  removed.value = null
  removeError.value = ''
  await load()
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

async function onRemove() {
  removeError.value = ''
  removed.value = null
  const result = await removeAddressPosts(
    investigated.value,
    hoursDraft.value,
    reasonDraft.value,
  )
  if (!result.ok) {
    removeError.value = result.error
    return
  }
  removed.value = result.value.removed
  currentPage.value = 1
  await load()
}
</script>

<template>
  <main>
    <h1>Addresses</h1>
    <p v-if="denied">Only a moderator can investigate an address.</p>
    <template v-else-if="isModerator">
      <form @submit.prevent="onInvestigate">
        <label>
          Address
          <input v-model="addrDraft" name="investigate-addr" type="text" />
        </label>
        <button type="submit">Investigate</button>
      </form>

      <p v-if="loadError" role="alert">{{ loadError }}</p>

      <ul>
        <li v-for="(post, index) in posts" :key="`${post.at}-${index}`">
          <span>{{ post.username }}</span>
          <span>{{ post.client ?? 'unknown client' }}</span>
          <span>{{ post.at }}</span>
        </li>
      </ul>
      <p v-if="listed && posts.length === 0 && !loadError">No posts from that address.</p>

      <nav v-if="page && posts.length > 0">
        <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
        <span>Page {{ page.number }} of {{ page.totalPages }}</span>
        <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
      </nav>

      <form v-if="listed" @submit.prevent="onRemove">
        <label>
          Hours
          <input v-model.number="hoursDraft" name="remove-hours" type="number" />
        </label>
        <label>
          Reason
          <input v-model="reasonDraft" name="remove-reason" type="text" />
        </label>
        <button type="submit">Remove posts</button>
        <p v-if="removed !== null" role="status">Removed {{ removed }} posts.</p>
        <p v-if="removeError" role="alert">{{ removeError }}</p>
      </form>
    </template>
  </main>
</template>
