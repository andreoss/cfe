<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { changePassword, deregister } from '@/api/client'

const auth = useAuthStore()
const router = useRouter()

const currentPassword = ref('')
const newPassword = ref('')
const formError = ref('')
const changed = ref(false)
const confirming = ref(false)
const deregisterError = ref('')

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
