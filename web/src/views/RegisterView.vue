<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { validateUsername, validateEmail, validatePassword } from '@/api/validation'

const username = ref('')
const email = ref('')
const password = ref('')
const formError = ref('')
const challengeNeeded = ref(false)
const challengeAnswer = ref('')
const auth = useAuthStore()
const router = useRouter()

async function onSubmit() {
  formError.value = ''
  const usernameCheck = validateUsername(username.value)
  if (!usernameCheck.ok) {
    formError.value = usernameCheck.error
    return
  }
  const emailCheck = validateEmail(email.value)
  if (!emailCheck.ok) {
    formError.value = emailCheck.error
    return
  }
  const passwordCheck = validatePassword(password.value)
  if (!passwordCheck.ok) {
    formError.value = passwordCheck.error
    return
  }
  const result = await auth.doRegister(
    username.value,
    email.value,
    password.value,
    challengeAnswer.value,
  )
  if (!result.ok) {
    if (result.status === 428 && !challengeNeeded.value) {
      challengeNeeded.value = true
      challengeAnswer.value = ''
      return
    }
    formError.value = result.error
    return
  }
  router.push('/')
}
</script>

<template>
  <main>
    <h1>Register</h1>
    <form @submit.prevent="onSubmit">
      <label>
        Username
        <input v-model="username" name="username" type="text" />
      </label>
      <label>
        Email
        <input v-model="email" name="email" type="email" />
      </label>
      <label>
        Password
        <input v-model="password" name="password" type="password" />
      </label>
      <template v-if="challengeNeeded">
        <p role="status">Answer the challenge to continue.</p>
        <label>
          Challenge
          <input v-model="challengeAnswer" name="challenge-answer" type="text" />
        </label>
      </template>
      <p v-if="formError" role="alert">{{ formError }}</p>
      <button type="submit">Create account</button>
    </form>
  </main>
</template>
