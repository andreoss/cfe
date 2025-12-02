<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  getTopicHistory,
  getTopicDifference,
  type DifferenceLine,
  type Version,
} from '@/api/client'

const props = defineProps<{ id: string }>()

const versions = ref<Version[]>([])
const loadError = ref('')
const openVersion = ref('')
const difference = ref<DifferenceLine[]>([])
const differenceError = ref('')

async function load() {
  loadError.value = ''
  openVersion.value = ''
  difference.value = []
  differenceError.value = ''
  const result = await getTopicHistory(props.id)
  if (result.ok) {
    versions.value = result.value
  } else {
    versions.value = []
    loadError.value = result.error
  }
}

async function showDifference(versionId: string) {
  differenceError.value = ''
  difference.value = []
  openVersion.value = versionId
  const result = await getTopicDifference(props.id, versionId)
  if (openVersion.value !== versionId) return
  if (result.ok) {
    difference.value = result.value
  } else {
    differenceError.value = result.error
  }
}

watch(() => props.id, load, { immediate: true })
</script>

<template>
  <main>
    <h1>History</h1>
    <p><RouterLink :to="`/t/${id}`">Back to topic</RouterLink></p>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="version in versions" :key="version.id">
        <span v-if="version.title" class="version-title">{{ version.title }}</span>
        <span class="version-editor">{{ version.editor }}</span>
        <span class="version-time">{{ version.writtenAt }}</span>
        <button type="button" @click="showDifference(version.id)">What changed</button>
        <template v-if="openVersion === version.id">
          <p v-if="differenceError" role="alert">{{ differenceError }}</p>
          <ul class="difference">
            <li v-for="(entry, index) in difference" :key="index" :class="entry.kind">{{ entry.line }}</li>
          </ul>
        </template>
      </li>
    </ul>
    <p v-if="versions.length === 0 && !loadError">No earlier versions.</p>
  </main>
</template>

<style scoped>
.version-title {
  font-weight: 650;
  font-size: var(--step-1);
}

.version-editor {
  font-weight: 550;
  color: var(--ink-soft);
}

.version-time {
  color: var(--ink-faint);
  font-size: var(--step-small);
  font-variant-numeric: tabular-nums;
}

.difference {
  list-style: none;
  padding: var(--gap-2);
  margin-top: var(--gap-3);
  background: var(--ground-soft);
  border: 1px solid var(--edge-soft);
  border-radius: var(--round);
  display: flex;
  flex-direction: column;
  gap: 1px;
  overflow-x: auto;
}
</style>
