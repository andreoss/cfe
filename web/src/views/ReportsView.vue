<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { listReports, closeReport, type PageInfo, type Report } from '@/api/client'

const auth = useAuthStore()
const reports = ref<Report[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')
const closeError = ref('')

const isModerator = computed(() => auth.currentUser?.role === 'moderator')
const denied = computed(() => auth.checked && !isModerator.value)

const ordered = computed(() =>
  [...reports.value].sort((a, b) => b.createdAt.localeCompare(a.createdAt)),
)

async function load() {
  if (!isModerator.value) {
    reports.value = []
    page.value = null
    return
  }
  loadError.value = ''
  const result = await listReports(currentPage.value)
  if (!result.ok) {
    loadError.value = result.error
    return
  }
  reports.value = result.value.items
  page.value = result.value.page
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

async function onClose(id: string) {
  closeError.value = ''
  const result = await closeReport(id)
  if (!result.ok) {
    closeError.value = result.error
    return
  }
  await load()
}

watch(() => auth.currentUser, load, { immediate: true })
</script>

<template>
  <main>
    <h1>Reports</h1>
    <p v-if="denied">Only a moderator can review reports.</p>
    <template v-else-if="isModerator">
      <p v-if="loadError" role="alert">{{ loadError }}</p>
      <p v-if="closeError" role="alert">{{ closeError }}</p>
      <ul>
        <li v-for="report in ordered" :key="report.id">
          <span>{{ report.reporterUsername }}</span>
          <span>{{ report.kind }}</span>
          <span>{{ report.reason }}</span>
          <RouterLink :to="`/t/${report.topicId}`">{{ report.topicId }}</RouterLink>
          <button type="button" @click="onClose(report.id)">Close report</button>
        </li>
      </ul>
      <p v-if="ordered.length === 0 && !loadError">No open reports.</p>

      <nav v-if="page && ordered.length > 0">
        <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
        <span>Page {{ page.number }} of {{ page.totalPages }}</span>
        <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
      </nav>
    </template>
  </main>
</template>

<style scoped>
main > ul {
  max-width: none;
  gap: var(--gap-1);
}

main > ul > li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
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
  padding: 0 var(--gap-2);
  border: 1px solid var(--edge);
  border-radius: 999px;
  color: var(--ink-soft);
  font-size: var(--step-tiny);
  white-space: nowrap;
}

main > ul > li > span:nth-child(3) {
  color: var(--ink-soft);
  overflow-wrap: anywhere;
}

main > ul > li > a {
  font-family: ui-monospace, 'SFMono-Regular', 'Cascadia Mono', Menlo, monospace;
  font-size: var(--step-tiny);
}

main > ul > li > button {
  margin-left: auto;
  padding-left: var(--gap-4);
  padding-right: var(--gap-4);
  border-color: var(--danger);
  color: var(--danger);
}

main > ul > li > button:hover:not(:disabled) {
  background: var(--danger-soft);
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

@media (max-width: 40rem) {
  main > ul > li > button {
    margin-left: 0;
    margin-top: var(--gap-2);
  }
}
</style>
