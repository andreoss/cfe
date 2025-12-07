<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useDisplayStore } from '@/stores/display'
import {
  changePassword,
  deregister,
  getMyWarnings,
  acknowledgeWarnings,
  requestEmailChange,
  confirmEmailChange,
  listAddressBlocks,
  blockAddress,
  liftAddressBlock,
  type Warning,
  type AddressBlock,
} from '@/api/client'

const auth = useAuthStore()
const display = useDisplayStore()
const router = useRouter()

const currentPassword = ref('')
const newPassword = ref('')
const formError = ref('')
const changed = ref(false)
const confirming = ref(false)
const deregisterError = ref('')
const confirmingEndAll = ref(false)
const endAllError = ref('')
const warnings = ref<Warning[]>([])
const warningsError = ref('')
const newEmail = ref('')
const emailSent = ref(false)
const emailError = ref('')
const emailCode = ref('')
const emailUpdated = ref(false)
const emailCodeError = ref('')

async function loadWarnings() {
  warningsError.value = ''
  const result = await getMyWarnings()
  if (!result.ok) {
    warningsError.value = result.error
    return
  }
  warnings.value = result.value
}

onMounted(() => {
  loadWarnings()
})

const blocks = ref<AddressBlock[]>([])
const blockError = ref('')
const blockAddr = ref('')
const blockReason = ref('')
const blockDays = ref('')

const isModerator = () => auth.currentUser?.role === 'moderator'

async function loadBlocks() {
  if (!isModerator()) return
  blockError.value = ''
  const result = await listAddressBlocks()
  if (!result.ok) {
    blockError.value = result.error
    return
  }
  blocks.value = result.value
}

watch(
  () => auth.currentUser,
  () => {
    loadBlocks()
  },
  { immediate: true },
)

async function onBlock() {
  blockError.value = ''
  const days = blockDays.value.trim() === '' ? null : Number(blockDays.value)
  const result = await blockAddress(blockAddr.value.trim(), blockReason.value.trim(), days)
  if (!result.ok) {
    blockError.value = result.error
    return
  }
  blockAddr.value = ''
  blockReason.value = ''
  blockDays.value = ''
  await loadBlocks()
}

async function onLiftBlock(addr: string) {
  blockError.value = ''
  const result = await liftAddressBlock(addr)
  if (!result.ok) {
    blockError.value = result.error
    return
  }
  await loadBlocks()
}

async function onAcknowledge() {
  warningsError.value = ''
  const result = await acknowledgeWarnings()
  if (!result.ok) {
    warningsError.value = result.error
    return
  }
  await loadWarnings()
}

async function onChangeEmail() {
  emailError.value = ''
  emailSent.value = false
  const result = await requestEmailChange(newEmail.value)
  if (!result.ok) {
    emailError.value = result.error
    return
  }
  newEmail.value = ''
  emailSent.value = true
}

async function onConfirmEmail() {
  emailCodeError.value = ''
  emailUpdated.value = false
  const result = await confirmEmailChange(emailCode.value)
  if (!result.ok) {
    emailCodeError.value = result.error
    return
  }
  emailCode.value = ''
  emailUpdated.value = true
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

async function onEndAllSessions() {
  endAllError.value = ''
  const result = await auth.doEndAllSessions()
  if (!result.ok) {
    endAllError.value = result.error
    return
  }
  confirmingEndAll.value = false
  router.push('/')
}
</script>

<template>
  <main>
    <h1>Settings</h1>
    <section>
      <h2>Display</h2>
      <label>
        Theme
        <select v-model="display.theme" name="theme">
          <option value="system">Match my system</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </label>
      <label>
        Times
        <select v-model="display.timeStyle" name="time-style">
          <option value="relative">How long ago</option>
          <option value="exact">Date and time</option>
        </select>
      </label>
      <label>
        Spacing
        <select v-model="display.density" name="density">
          <option value="comfortable">Comfortable</option>
          <option value="compact">Compact</option>
        </select>
      </label>
      <p role="status">These are kept in this browser.</p>
    </section>
    <p v-if="!auth.currentUser">Sign in to manage your account.</p>
    <template v-else>
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
      <section v-if="isModerator()">
        <h2>Blocked addresses</h2>
        <ul>
          <li v-for="block in blocks" :key="block.addr">
            {{ block.addr }} – {{ block.reason }}
            <button type="button" @click="onLiftBlock(block.addr)">Lift</button>
          </li>
        </ul>
        <p v-if="blocks.length === 0">No blocked addresses.</p>
        <p v-if="blockError" role="alert">{{ blockError }}</p>
        <form @submit.prevent="onBlock">
          <label>
            Address
            <input v-model="blockAddr" name="block-addr" type="text" />
          </label>
          <label>
            Reason
            <input v-model="blockReason" name="block-reason" type="text" />
          </label>
          <label>
            Days
            <input v-model="blockDays" name="block-days" type="number" min="0" />
          </label>
          <button type="submit">Block address</button>
        </form>
      </section>
      <form @submit.prevent="onChangeEmail">
        <label>
          New address
          <input v-model="newEmail" name="new-email" type="email" />
        </label>
        <p v-if="emailSent" role="status">Confirmation sent to the new address.</p>
        <p v-if="emailError" role="alert">{{ emailError }}</p>
        <button type="submit">Change address</button>
      </form>
      <form @submit.prevent="onConfirmEmail">
        <label>
          Confirmation code
          <input v-model="emailCode" name="email-code" type="text" />
        </label>
        <p v-if="emailUpdated" role="status">Address updated.</p>
        <p v-if="emailCodeError" role="alert">{{ emailCodeError }}</p>
        <button type="submit">Confirm address change</button>
      </form>
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
      <div class="danger-zone">
        <template v-if="confirmingEndAll">
          <p>This signs you out on every device.</p>
          <p v-if="endAllError" role="alert">{{ endAllError }}</p>
          <button class="grave" type="button" @click="onEndAllSessions">Confirm end all sessions</button>
          <button class="mild" type="button" @click="confirmingEndAll = false">Cancel</button>
        </template>
        <button v-else class="grave" type="button" @click="confirmingEndAll = true">End all sessions</button>
        <template v-if="confirming">
          <p>This removes your account for good.</p>
          <p v-if="deregisterError" role="alert">{{ deregisterError }}</p>
          <button class="grave" type="button" @click="onDeregister">Confirm deregister</button>
          <button class="mild" type="button" @click="confirming = false">Cancel</button>
        </template>
        <button v-else class="grave" type="button" @click="confirming = true">Deregister account</button>
      </div>
    </template>
  </main>
</template>

<style scoped>
main {
  gap: var(--gap-5);
}

main > h1 {
  padding-bottom: var(--gap-2);
  border-bottom: 1px solid var(--edge);
  margin-bottom: 0;
}

main > p:not([role]) {
  padding: var(--gap-4);
  border: 1px dashed var(--edge);
  border-radius: var(--round-large);
  background: var(--ground);
  color: var(--ink-soft);
}

main > section,
main > form {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gap-3);
  padding: var(--gap-4) var(--gap-5);
  border: 1px solid var(--edge-soft);
  border-radius: var(--round-large);
  background: var(--ground);
  box-shadow: var(--shadow);
}

main > section > *,
main > form > * {
  width: 100%;
}

main > section > button,
main > form > button {
  width: auto;
}

section > h2 {
  padding-bottom: var(--gap-2);
  border-bottom: 1px solid var(--edge-soft);
  font-size: var(--step-1);
}

section > p:not([role]) {
  color: var(--ink-soft);
  font-size: var(--step-small);
}

section ul {
  list-style: none;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--gap-2);
}

section li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--gap-2) var(--gap-4);
  padding: var(--gap-2) var(--gap-3);
  border: 1px solid var(--edge-soft);
  border-radius: var(--round);
  background: var(--ground-soft);
  font-size: var(--step-small);
}

section form {
  gap: var(--gap-3);
  padding: var(--gap-3);
  border: 1px solid var(--edge);
  border-radius: var(--round);
  background: var(--ground-soft);
}

p[role='status'] {
  padding: var(--gap-2) var(--gap-3);
  border-left: 3px solid var(--good);
  border-radius: var(--round);
  background: var(--good-soft);
  color: var(--good);
  font-size: var(--step-small);
  font-weight: 550;
}

p[role='alert'] {
  border-left: 3px solid var(--danger);
}

.danger-zone {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gap-4);
  margin-top: var(--gap-4);
  padding: var(--gap-4) var(--gap-5);
  border: 1px solid var(--danger);
  border-radius: var(--round-large);
  background: var(--danger-soft);
}

.danger-zone p:not([role]) {
  color: var(--ink-soft);
  font-size: var(--step-small);
}

.danger-zone p[role='alert'] {
  align-self: stretch;
  border: 1px solid var(--danger);
  background: var(--ground);
}

.danger-zone button.grave {
  border-color: var(--danger);
  background: var(--ground);
  color: var(--danger);
}

.danger-zone button.grave:hover:not(:disabled) {
  background: var(--danger);
  color: var(--ground);
}

.danger-zone button.mild {
  border-color: var(--edge);
  background: transparent;
  color: var(--ink-soft);
}

@media (max-width: 40rem) {
  main > section,
  main > form,
  .danger-zone {
    padding: var(--gap-4) var(--gap-4);
  }
}
</style>
