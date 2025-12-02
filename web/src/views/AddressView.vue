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
      <form class="probe" @submit.prevent="onInvestigate">
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

      <form v-if="listed" class="removal" @submit.prevent="onRemove">
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

<style scoped>
.probe {
  flex-direction: row;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: var(--gap-3);
}

.probe label {
  flex: 1 1 16rem;
}

.probe button {
  align-self: auto;
}

main > ul {
  max-width: none;
  gap: var(--gap-1);
}

main > ul > li {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--gap-1) var(--gap-4);
  padding: var(--gap-2) var(--gap-3);
  border-radius: var(--round);
  box-shadow: none;
  font-size: var(--step-small);
}

main > ul > li:hover {
  background: var(--ground-soft);
}

main > ul > li > span:first-child {
  min-width: 12ch;
  font-weight: 600;
  font-size: var(--step-0);
}

main > ul > li > span:nth-child(2) {
  color: var(--ink-soft);
}

main > ul > li > span:last-child {
  margin-left: auto;
  font-family: ui-monospace, 'SFMono-Regular', 'Cascadia Mono', Menlo, monospace;
  font-size: var(--step-tiny);
  color: var(--ink-faint);
}

main > p:not([role]) {
  color: var(--ink-faint);
  font-size: var(--step-small);
  padding: var(--gap-3) 0;
}

main > nav {
  display: flex;
  align-items: center;
  gap: var(--gap-3);
  font-size: var(--step-small);
  color: var(--ink-soft);
}

.removal {
  margin-top: var(--gap-4);
  border: 1px solid var(--danger);
  background: var(--danger-soft);
}

.removal input {
  background: var(--ground);
}

.removal button[type='submit'] {
  margin-top: var(--gap-2);
  padding-left: var(--gap-4);
  padding-right: var(--gap-4);
  background: var(--danger);
  border-color: var(--danger);
  color: var(--ground);
}

.removal p[role='status'] {
  padding: var(--gap-2) var(--gap-3);
  border-radius: var(--round);
  background: var(--good-soft);
  color: var(--good);
  font-size: var(--step-small);
  font-weight: 550;
}

.removal p[role='alert'] {
  background: var(--ground);
}
</style>
