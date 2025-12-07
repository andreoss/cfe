<script setup lang="ts">
import { useRoute } from 'vue-router'

const route = useRoute()
</script>

<template>
  <main>
    <h1>There is nothing here</h1>
    <p>Nothing answers to <code class="asked">{{ route.fullPath }}</code>.</p>
    <p>
      It may have been removed, or the address may have a typo in it. From here you can
      <RouterLink to="/">go back to the sections</RouterLink> or
      <RouterLink to="/search">search for what you wanted</RouterLink>.
    </p>
  </main>
</template>

<style scoped>
.asked {
  overflow-wrap: anywhere;
}
</style>
