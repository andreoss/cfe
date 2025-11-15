<script setup lang="ts">
import { REACTION_KINDS, type ReactionSummary } from '@/api/client'

defineProps<{
  summary: ReactionSummary | null
  disabled: boolean
  onPick: (kind: string) => void
}>()
</script>

<template>
  <div class="reaction-bar">
    <button
      v-for="kind in REACTION_KINDS"
      :key="kind"
      type="button"
      :disabled="disabled"
      :class="{ mine: summary?.mine === kind }"
      @click="onPick(kind)"
    >
      {{ kind }}
      {{ summary?.counts.find((c: { kind: string; count: number }) => c.kind === kind)?.count ?? 0 }}
    </button>
  </div>
</template>
