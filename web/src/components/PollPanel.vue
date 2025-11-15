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
