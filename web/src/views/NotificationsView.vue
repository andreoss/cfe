<script setup lang="ts">
import { ref } from 'vue'
import { getNotifications, markNotificationRead, type Notification } from '@/api/client'

const notifications = ref<Notification[]>([])
const loadError = ref('')

async function load() {
  loadError.value = ''
  const result = await getNotifications()
  if (result.ok) {
    notifications.value = result.value
  } else {
    loadError.value = result.error
  }
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
  </main>
</template>
