import { defineStore } from 'pinia'
import { ref } from 'vue'
import { register, signIn, signOut, endAllSessions, me, type User } from '@/api/client'

export const useAuthStore = defineStore('auth', () => {
  const currentUser = ref<User | null>(null)
  const checked = ref(false)

  async function doRegister(
    username: string,
    email: string,
    password: string,
    challenge?: string,
  ) {
    const result = await register(username, email, password, challenge)
    if (result.ok) currentUser.value = result.value
    return result
  }

  async function doSignIn(username: string, password: string) {
    const result = await signIn(username, password)
    if (result.ok) currentUser.value = result.value
    return result
  }

  function clearSession() {
    currentUser.value = null
  }

  async function doSignOut() {
    await signOut()
    clearSession()
  }

  async function doEndAllSessions() {
    const result = await endAllSessions()
    if (result.ok) clearSession()
    return result
  }

  async function checkSession() {
    const result = await me()
    currentUser.value = result.ok ? result.value : null
    checked.value = true
  }

  return {
    currentUser,
    checked,
    doRegister,
    doSignIn,
    doSignOut,
    doEndAllSessions,
    checkSession,
  }
})
