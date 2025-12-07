<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { getSections, listAddressBlocks, listReports, runMaintenance } from '@/api/client'

const auth = useAuthStore()

const openReports = ref<number | null>(null)
const blockedAddresses = ref<number | null>(null)
const sections = ref<number | null>(null)
const loadError = ref('')
const maintenance = ref('')
const maintenanceError = ref('')

const isModerator = () => auth.currentUser?.role === 'moderator'

async function load() {
  if (!isModerator()) return
  loadError.value = ''
  const reports = await listReports(1, 1)
  if (reports.ok) openReports.value = reports.value.page.total
  else loadError.value = reports.error
  const blocks = await listAddressBlocks()
  if (blocks.ok) blockedAddresses.value = blocks.value.length
  const found = await getSections()
  if (found.ok) sections.value = found.value.length
}

async function onRunMaintenance() {
  maintenanceError.value = ''
  maintenance.value = ''
  const result = await runMaintenance()
  if (!result.ok) {
    maintenanceError.value = result.error
    return
  }
  maintenance.value = `Blocked ${result.value.blocked}, dropped ${result.value.dropped}.`
  await load()
}

function shown(value: number | null) {
  return value === null ? '—' : String(value)
}

watch(() => auth.currentUser, load, { immediate: true })
</script>

<template>
  <main>
    <h1>Operator</h1>
    <p v-if="!isModerator()">Only a moderator can run the board.</p>
    <template v-else>
      <p v-if="loadError" role="alert">{{ loadError }}</p>
      <ul class="state">
        <li>
          <RouterLink to="/reports">Open reports</RouterLink>
          <strong class="count" data-of="reports">{{ shown(openReports) }}</strong>
        </li>
        <li>
          <RouterLink to="/addresses">Blocked addresses</RouterLink>
          <strong class="count" data-of="addresses">{{ shown(blockedAddresses) }}</strong>
        </li>
        <li>
          <RouterLink to="/section-settings">Sections</RouterLink>
          <strong class="count" data-of="sections">{{ shown(sections) }}</strong>
        </li>
      </ul>

      <section>
        <h2>Maintenance</h2>
        <button type="button" @click="onRunMaintenance">Run maintenance</button>
        <p v-if="maintenance" role="status">{{ maintenance }}</p>
        <p v-if="maintenanceError" role="alert">{{ maintenanceError }}</p>
      </section>
    </template>
  </main>
</template>

<style scoped>
.state {
  list-style: none;
  padding: 0;
  display: grid;
  gap: var(--gap-2);
}

.state li {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: var(--gap-4);
  padding: var(--gap-3);
  border: 1px solid var(--edge-soft);
  border-radius: var(--round);
  background: var(--ground-soft);
}

.count {
  font-size: var(--step-2);
  font-variant-numeric: tabular-nums;
}
</style>
