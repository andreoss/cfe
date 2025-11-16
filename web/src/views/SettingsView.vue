<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import {
  changePassword,
  deregister,
  getMyWarnings,
  acknowledgeWarnings,
  type Warning,
} from '@/api/client'

const auth = useAuthStore()
const router = useRouter()

const currentPassword = ref('')
const newPassword = ref('')
const formError = ref('')
const changed = ref(false)
const confirming = ref(false)
const deregisterError = ref('')
const warnings = ref<Warning[]>([])
const warningsError = ref('')

async function loadWarnings() {
  warningsError.value = ''
  const result = await getMyWarnings()
  if (!result.ok) {
    warningsError.value = result.error
    return
  }
  warnings.value = result.value
}

onMounted(loadWarnings)

async function onAcknowledge() {
  warningsError.value = ''
  const result = await acknowledgeWarnings()
  if (!result.ok) {
    warningsError.value = result.error
    return
  }
  await loadWarnings()
}

async function onChangePassword() {
  formError.value = ''
  changed.value = false
  const result = await changePassword(currentPassword.value, newPassword.value)
  if (!result.ok) {
    formError.value = result.error
    return
  }
  currentPassword.value = ''
  newPassword.value = ''
  changed.value = true
}

async function onDeregister() {
  deregisterError.value = ''
  const result = await deregister()
  if (!result.ok) {
    deregisterError.value = result.error
    return
  }
  confirming.value = false
  await auth.doSignOut()
  router.push('/')
}
</script>

<template>
  <main>
    <p v-if="!auth.currentUser">Sign in to manage your account.</p>
    <template v-else>
      <h1>Settings</h1>
      <section>
        <p v-if="warnings.length === 0">No warnings.</p>
        <template v-else>
          <ul>
            <li v-for="warning in warnings" :key="warning.id">{{ warning.reason }}</li>
          </ul>
          <button type="button" @click="onAcknowledge">Acknowledge warnings</button>
        </template>
        <p v-if="warningsError" role="alert">{{ warningsError }}</p>
      </section>
      <form @submit.prevent="onChangePassword">
        <label>
          Current password
          <input v-model="currentPassword" name="current-password" type="password" />
        </label>
        <label>
          New password
          <input v-model="newPassword" name="new-password" type="password" />
        </label>
        <p v-if="changed" role="status">Password changed.</p>
        <p v-if="formError" role="alert">{{ formError }}</p>
        <button type="submit">Change password</button>
      </form>
      <template v-if="confirming">
        <p>This removes your account for good.</p>
        <p v-if="deregisterError" role="alert">{{ deregisterError }}</p>
        <button type="button" @click="onDeregister">Confirm deregister</button>
        <button type="button" @click="confirming = false">Cancel</button>
      </template>
      <button v-else type="button" @click="confirming = true">Deregister account</button>
    </template>
  </main>
</template>
