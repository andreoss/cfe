<script setup lang="ts">
import { ref } from 'vue'
import { requestPasswordReset, confirmPasswordReset } from '@/api/client'

const email = ref('')
const requestSent = ref(false)
const requestError = ref('')
const code = ref('')
const password = ref('')
const changed = ref(false)
const confirmError = ref('')

async function onRequest() {
  requestError.value = ''
  requestSent.value = false
  const result = await requestPasswordReset(email.value)
  if (!result.ok) {
    requestError.value = result.error
    return
  }
  requestSent.value = true
}

async function onConfirm() {
  confirmError.value = ''
  changed.value = false
  const result = await confirmPasswordReset(code.value, password.value)
  if (!result.ok) {
    confirmError.value = result.error
    return
  }
  code.value = ''
  password.value = ''
  changed.value = true
}
</script>

<template>
  <main>
    <h1>Forgot password</h1>
    <form @submit.prevent="onRequest">
      <label>
        Email
        <input v-model="email" name="reset-email" type="email" />
      </label>
      <p v-if="requestSent" role="status">If that address has an account, a reset code is on its way.</p>
      <p v-if="requestError" role="alert">{{ requestError }}</p>
      <button type="submit">Send reset code</button>
    </form>
    <form @submit.prevent="onConfirm">
      <label>
        Reset code
        <input v-model="code" name="reset-code" type="text" />
      </label>
      <label>
        New password
        <input v-model="password" name="reset-password" type="password" />
      </label>
      <p v-if="changed" role="status">Password changed. You can sign in now.</p>
      <p v-if="confirmError" role="alert">{{ confirmError }}</p>
      <button type="submit">Set new password</button>
    </form>
  </main>
</template>
