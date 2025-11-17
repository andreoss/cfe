<script setup lang="ts">
import { ref } from 'vue'
import { activateAccount } from '@/api/client'

const code = ref('')
const activated = ref(false)
const formError = ref('')

async function onSubmit() {
  formError.value = ''
  activated.value = false
  const result = await activateAccount(code.value)
  if (!result.ok) {
    formError.value = result.error
    return
  }
  code.value = ''
  activated.value = true
}
</script>

<template>
  <main>
    <h1>Activate account</h1>
    <form @submit.prevent="onSubmit">
      <label>
        Activation code
        <input v-model="code" name="activation-code" type="text" />
      </label>
      <p v-if="activated" role="status">Address confirmed.</p>
      <p v-if="formError" role="alert">{{ formError }}</p>
      <button type="submit">Confirm address</button>
    </form>
  </main>
</template>
