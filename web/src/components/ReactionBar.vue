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

<style scoped>
.reaction-bar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gap-2);
}

.reaction-bar button {
  padding: 0.2rem 0.6rem;
  border-radius: 999px;
  font-size: var(--step-tiny);
  color: var(--ink-soft);
}

.reaction-bar button.mine {
  background: var(--accent-soft);
  border-color: var(--accent);
  color: var(--accent);
}
</style>
