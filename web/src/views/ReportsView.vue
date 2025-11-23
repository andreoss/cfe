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
