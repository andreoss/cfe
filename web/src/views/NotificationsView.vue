<script setup lang="ts">
import { ref } from 'vue'
import {
  getNotifications,
  markNotificationRead,
  type Notification,
  type PageInfo,
} from '@/api/client'

const notifications = ref<Notification[]>([])
const page = ref<PageInfo | null>(null)
const currentPage = ref(1)
const loadError = ref('')

async function load() {
  loadError.value = ''
  const result = await getNotifications(currentPage.value)
  if (result.ok) {
    notifications.value = result.value.items
    page.value = result.value.page
  } else {
    loadError.value = result.error
  }
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

async function onMarkRead(notification: Notification) {
  const result = await markNotificationRead(notification.id)
  if (result.ok) {
    await load()
  } else {
    loadError.value = result.error
  }
}

load()
</script>

<template>
  <main>
    <h1>Notifications</h1>
    <p v-if="loadError" role="alert">{{ loadError }}</p>
    <ul>
      <li v-for="notification in notifications" :key="notification.id">
        <span>{{ notification.kind === 'watch' ? 'Watched topic' : 'Reply' }}</span>
        {{ notification.actorUsername }} replied
        <RouterLink :to="`/t/${notification.topicId}`">{{ notification.topicTitle }}</RouterLink>
        <button
          v-if="!notification.read"
          type="button"
          @click="onMarkRead(notification)"
        >
          Mark read
        </button>
      </li>
    </ul>
    <p v-if="notifications.length === 0 && !loadError">No notifications.</p>

    <nav v-if="page && page.totalPages > 1">
      <button type="button" :disabled="!page.hasPrevious" @click="previousPage">Previous</button>
      <span>Page {{ page.number }} of {{ page.totalPages }}</span>
      <button type="button" :disabled="!page.hasNext" @click="nextPage">Next</button>
    </nav>
  </main>
</template>
