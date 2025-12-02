<script setup lang="ts">
import type { Poll } from '@/api/client'

defineProps<{
  poll: Poll | null
  canVote: boolean
  onVote: (optionId: string) => void
}>()
</script>

<template>
  <div v-if="poll" class="poll-panel">
    <h3>{{ poll.question }}</h3>
    <button
      v-for="option in poll.options"
      :key="option.id"
      type="button"
      :disabled="!canVote"
      :class="{ mine: poll.mine === option.id }"
      @click="onVote(option.id)"
    >
      {{ option.text }} ({{ option.votes }})
    </button>
    <p class="total">Total votes: {{ poll.totalVotes }}</p>
  </div>
</template>

<style scoped>
.poll-panel {
  padding: var(--gap-4);
  background: var(--ground);
  border: 1px solid var(--edge-soft);
  border-radius: var(--round-large);
  display: flex;
  flex-direction: column;
  gap: var(--gap-3);
  max-width: var(--reading);
}

.poll-panel ul {
  list-style: none;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--gap-2);
}

.poll-panel li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gap-3);
  padding: var(--gap-2) var(--gap-3);
  background: var(--ground-soft);
  border-radius: var(--round);
  font-size: var(--step-small);
}
</style>
