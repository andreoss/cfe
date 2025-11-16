<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { avatarUrl } from '@/api/client'

const props = withDefaults(
  defineProps<{ username: string; size?: number; version?: number }>(),
  { size: 24, version: 0 },
)

const failed = ref(false)

const src = computed(() =>
  props.version > 0
    ? `${avatarUrl(props.username)}?v=${props.version}`
    : avatarUrl(props.username),
)

watch(src, () => {
  failed.value = false
})
</script>

<template>
  <img
    v-if="!failed"
    class="avatar"
    :src="src"
    :width="size"
    :height="size"
    :alt="username"
    @error="failed = true"
  />
</template>
