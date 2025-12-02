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
      <li
        v-for="notification in notifications"
        :key="notification.id"
        :class="notification.read ? 'seen' : 'unread'"
      >
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

<style scoped>
main > ul {
  gap: var(--gap-1);
}

main > ul > li {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--gap-1) var(--gap-2);
  padding: var(--gap-2) var(--gap-3);
  border-radius: var(--round);
  border-left: 3px solid transparent;
  box-shadow: none;
  font-size: var(--step-small);
}

main > ul > li > span:first-child {
  padding: 0 var(--gap-2);
  border-radius: 999px;
  background: var(--ground-sunk);
  color: var(--ink-soft);
  font-size: var(--step-tiny);
  font-weight: 600;
  white-space: nowrap;
}

main > ul > li > a {
  font-weight: 600;
}

main > ul > li > button {
  margin-left: auto;
  padding-left: var(--gap-3);
  padding-right: var(--gap-3);
  font-size: var(--step-tiny);
}

li.unread {
  border-left-color: var(--accent);
  background: var(--accent-soft);
  font-weight: 600;
}

li.unread > span:first-child {
  background: var(--accent);
  color: var(--accent-ink);
}

li.seen {
  color: var(--ink-soft);
  font-weight: 400;
}

main > p:not([role]) {
  color: var(--ink-faint);
  font-size: var(--step-small);
  padding: var(--gap-3) 0;
}

main > nav {
  display: flex;
  align-items: center;
  gap: var(--gap-3);
  font-size: var(--step-small);
  color: var(--ink-soft);
}
</style>
