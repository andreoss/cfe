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
    formError.value =
      result.status === 429 ? 'Too many attempts. Try again later.' : result.error
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

<style scoped>
main {
  max-width: 24rem;
  gap: var(--gap-3);
  padding-top: var(--gap-6);
}

main > h1 {
  margin-bottom: 0;
  font-size: var(--step-2);
  text-align: center;
}

form {
  gap: var(--gap-4);
  padding: var(--gap-5);
  box-shadow: var(--shadow);
}

form label {
  color: var(--ink);
}

form p[role='alert'] {
  border-left: 3px solid var(--danger);
}

form button[type='submit'] {
  align-self: stretch;
  margin-top: var(--gap-1);
  padding: 0.55rem 0.85rem;
  font-size: var(--step-0);
}

main > a {
  align-self: center;
  color: var(--ink-soft);
  font-size: var(--step-small);
}

main > a:hover {
  color: var(--accent);
}
</style>
