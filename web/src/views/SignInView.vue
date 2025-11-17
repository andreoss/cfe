<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const username = ref('')
const password = ref('')
const formError = ref('')
const auth = useAuthStore()
const router = useRouter()

async function onSubmit() {
  formError.value = ''
  const result = await auth.doSignIn(username.value, password.value)
  if (!result.ok) {
    formError.value = result.error
    return
  }
  router.push('/')
}
</script>

<template>
  <main>
    <h1>Sign in</h1>
    <form @submit.prevent="onSubmit">
      <label>
        Username
        <input v-model="username" name="username" type="text" />
      </label>
      <label>
        Password
        <input v-model="password" name="password" type="password" />
      </label>
      <p v-if="formError" role="alert">{{ formError }}</p>
      <button type="submit">Sign in</button>
    </form>
    <RouterLink to="/forgot-password">Forgot password?</RouterLink>
  </main>
</template>
