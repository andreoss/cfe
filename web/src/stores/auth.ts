import { defineStore } from 'pinia'
import { ref } from 'vue'
import { register, signIn, type User } from '@/api/client'

export const useAuthStore = defineStore('auth', () => {
  const currentUser = ref<User | null>(null)

  async function doRegister(username: string, email: string, password: string) {
    const result = await register(username, email, password)
    if (result.ok) currentUser.value = result.value
    return result
  }

  async function doSignIn(username: string, password: string) {
    const result = await signIn(username, password)
    if (result.ok) currentUser.value = result.value
    return result
  }

  function signOut() {
    currentUser.value = null
  }

  return { currentUser, doRegister, doSignIn, signOut }
})
