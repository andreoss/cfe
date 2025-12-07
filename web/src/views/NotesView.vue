<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { getRemarks, type PageInfo, type Remark } from '@/api/client'

const auth = useAuthStore()
const remarks = ref<Remark[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')

async function load() {
  if (auth.currentUser === null) {
    remarks.value = []
    page.value = null
    loadError.value = ''
    return
  }
  loadError.value = ''
  const result = await getRemarks(currentPage.value)
  if (result.ok) {
    remarks.value = result.value.items
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

watch(() => auth.currentUser, load, { immediate: true })
</script>

<template>
  <main>
    <p v-if="auth.checked && !auth.currentUser">Sign in to see your notes.</p>
    <template v-else>
      <h1>Notes</h1>
      <p v-if="loadError" role="alert">{{ loadError }}</p>
      <ul>
        <li v-for="remark in remarks" :key="remark.subjectUsername">
          <RouterLink :to="`/u/${remark.subjectUsername}`">{{
            remark.subjectUsername
          }}</RouterLink>
          <span>{{ remark.text }}</span>
        </li>
      </ul>
      <p v-if="remarks.length === 0 && !loadError">You have not written any notes.</p>

      <nav v-if="page && page.totalPages > 1">
        <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
        <span>Page {{ page.number }} of {{ page.totalPages }}</span>
        <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
      </nav>
    </template>
  </main>
</template>

<style scoped>
main > ul > li {
  display: grid;
  gap: var(--gap-1);
  padding: var(--gap-3) var(--gap-4);
}

main > ul > li a {
  justify-self: start;
  font-size: var(--step-small);
}

main > ul > li span {
  color: var(--ink-soft);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
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
