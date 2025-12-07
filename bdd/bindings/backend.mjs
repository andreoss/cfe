import { cookieJar } from './board.mjs'

const base = process.env.CLONE_URL ?? 'http://127.0.0.1:58080'

const people = {
  administrator: { name: 'admin', password: 'admin' },
  moderator: { name: 'admin', password: 'admin' },
  reader: { name: process.env.CLONE_READER ?? 'bddreader', password: 'correcthorse' },
}

export function board() {
  const jar = cookieJar()

  async function send(path, init = {}) {
    const response = await fetch(`${base}${path}`, {
      ...init,
      headers: {
        cookie: jar.header(),
        ...(init.body ? { 'content-type': 'application/json' } : {}),
        ...(init.headers ?? {}),
      },
      redirect: 'manual',
    })
    jar.take(response)
    return response
  }

  async function ensureReader() {
    const who = people.reader
    await send('/api/register', {
      method: 'POST',
      body: JSON.stringify({
        username: who.name,
        email: `${who.name}@example.com`,
        password: who.password,
      }),
    })
    jar.forget()
  }

  return {
    name: 'clone',

    async answering() {
      const response = await send('/api/sections')
      return response.status === 200
    },

    async signIn(role, password) {
      const who = people[role]
      if (!who) throw new Error(`this board has nobody called a ${role}`)
      if (role === 'reader') await ensureReader()
      await send('/api/sign-in', {
        method: 'POST',
        body: JSON.stringify({ username: who.name, password: password ?? who.password }),
      })
      return who.name
    },

    async signOut() {
      await send('/api/sign-out', { method: 'POST' })
      jar.forget()
    },

    async whoAmI() {
      const response = await send('/api/me')
      if (response.status !== 200) return null
      const body = await response.json()
      return body?.username ?? null
    },

    nameFor(role) {
      return people[role]?.name ?? null
    },
  }
}
