<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getInvitations,
  issueInvitation,
  type Invitation,
  type PageInfo,
} from '@/api/client'

const auth = useAuthStore()
const invitations = ref<Invitation[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')
const issueError = ref('')

async function load() {
  if (auth.currentUser === null) {
    invitations.value = []
    page.value = null
    loadError.value = ''
    return
  }
  loadError.value = ''
  const result = await getInvitations(currentPage.value)
  if (result.ok) {
    invitations.value = result.value.items
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

async function onIssue() {
  issueError.value = ''
  const result = await issueInvitation()
  if (!result.ok) {
    issueError.value =
      result.error === 'too many unused invitations'
        ? 'Too many unused invitations.'
        : result.error
    return
  }
  await goToPage(1)
}

watch(() => auth.currentUser, load, { immediate: true })
</script>

<template>
  <main>
    <p v-if="auth.checked && !auth.currentUser">Sign in to see your invitations.</p>
    <template v-else>
      <h1>Invitations</h1>
      <button type="button" @click="onIssue">Issue code</button>
      <p v-if="issueError" role="alert">{{ issueError }}</p>
      <p v-if="loadError" role="alert">{{ loadError }}</p>
      <ul>
        <li v-for="invitation in invitations" :key="invitation.code">
          <span class="invitation-code">{{ invitation.code }}</span>
          <span>{{ invitation.expiresAt }}</span>
          <span v-if="invitation.spent">Used by {{ invitation.spentBy }}</span>
          <span v-else>Unused</span>
        </li>
      </ul>
      <p v-if="invitations.length === 0 && !loadError">You have not issued any invitations.</p>

      <nav v-if="page && page.totalPages > 1">
        <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
        <span>Page {{ page.number }} of {{ page.totalPages }}</span>
        <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
      </nav>
    </template>
  </main>
</template>

<style scoped>
main > button {
  align-self: flex-start;
  padding: 0.45rem 1rem;
  background: var(--accent);
  border-color: var(--accent);
  color: var(--accent-ink);
}

main > button:hover:not(:disabled) {
  background: var(--accent);
  filter: brightness(1.08);
}

main > ul {
  padding-top: var(--gap-2);
  border-top: 1px solid var(--edge);
}

main > ul:empty {
  border-top: none;
  padding-top: 0;
}

main > ul > li {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--gap-1) var(--gap-4);
  padding: var(--gap-3);
}

.invitation-code {
  flex: 0 0 auto;
  padding: var(--gap-1) var(--gap-2);
  border: 1px solid var(--edge);
  border-radius: var(--round);
  background: var(--ground-sunk);
  font-size: var(--step-small);
  user-select: all;
}

main > ul > li > span:not(.invitation-code) {
  font-size: var(--step-small);
  color: var(--ink-soft);
}

main > ul > li > span:last-child {
  margin-left: auto;
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
</style>
