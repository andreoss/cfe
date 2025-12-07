import { cookieJar } from './board.mjs'

const base = process.env.CLONE_URL ?? 'http://127.0.0.1:58080'

const people = {
  administrator: { name: 'admin', password: 'admin' },
  moderator: { name: 'admin', password: 'admin' },
  reader: { name: process.env.CLONE_READER ?? 'bddreader', password: 'correcthorse' },
}

export function board() {
  const jar = cookieJar()
  let subject = null

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

    async sections() {
      const response = await send('/api/sections')
      const body = await response.json()
      return body.map((one) => one.slug)
    },

    async openSection() {
      const listed = await send('/api/sections')
      if (listed.status !== 200) return { found: false, names: '' }
      const sections = await listed.json()
      const first = sections[0]
      if (!first) return { found: false, names: '' }
      const opened = await send(`/api/sections/${first.slug}/topics`)
      return {
        found: opened.status === 200,
        names: first.title ?? first.slug ?? '',
      }
    },

    async ensureSubject() {
      const where = (await this.sections())[0]
      const listing = await send(`/api/sections/${where}/topics`)
      if (listing.status === 200) {
        const body = await listing.json()
        if ((body.items ?? []).length > 0) return true
      }
      await this.signIn('administrator')
      const made = await send(`/api/sections/${where}/topics`, {
        method: 'POST',
        body: JSON.stringify({
          title: 'Something worth reading',
          body: 'A subject put here so that anybody may read one.',
          tags: [],
        }),
      })
      if (made.status < 300) {
        const topic = await made.json()
        if (topic?.pending) {
          await send(`/api/topics/${topic.id}/commit`, { method: 'POST' })
        }
      }
      await this.signOut()
      return true
    },

    async openSubject() {
      const where = (await this.sections())[0]
      const response = await send(`/api/sections/${where}/topics`)
      if (response.status !== 200) return { found: false, title: '', author: '' }
      const body = await response.json()
      const first = (body.items ?? [])[0]
      if (!first) return { found: false, title: '', author: '' }
      return { found: true, title: first.title ?? '', author: first.author_username ?? '' }
    },

    async lookUpAccount(role) {
      const who = people[role]
      const response = await send(`/api/users/${encodeURIComponent(who.name)}`)
      if (response.status !== 200) return { found: false, names: '' }
      const body = await response.json()
      return { found: true, names: body.username ?? '' }
    },

    async lookUpMissingAccount() {
      const response = await send('/api/users/nobody-answers-to-this-name')
      return { found: response.status === 200 }
    },

    async openMissingSubject() {
      const response = await send('/api/topics/00000000-0000-0000-0000-000000000000')
      return { found: response.status === 200 }
    },

    async subjectToWriteOn() {
      const where = (await this.sections())[0]
      const response = await send(`/api/sections/${where}/topics`)
      const body = await response.json()
      const first = (body.items ?? [])[0]
      if (!first) throw new Error('the board offered nothing to write on')
      subject = { id: first.id }
      return subject.id
    },

    async addRemark(text) {
      if (!subject) await this.subjectToWriteOn()
      const response = await send(`/api/topics/${subject.id}/comments`, {
        method: 'POST',
        body: JSON.stringify({ body: text }),
      })
      return { accepted: response.status >= 200 && response.status < 300 }
    },

    async remarksOn(text) {
      if (!subject) return { present: false, author: '' }
      const response = await send(`/api/topics/${subject.id}/comments`)
      if (response.status !== 200) return { present: false, author: '' }
      const body = await response.json()
      const found = (body.items ?? []).find(
        (one) => (one.body ?? '').includes(text) && !one.deleted,
      )
      if (!found) return { present: false, author: '' }
      return { present: true, author: found.author_username ?? '' }
    },

    async findRemark(text) {
      const response = await send(`/api/topics/${subject.id}/comments`)
      if (response.status !== 200) return null
      const body = await response.json()
      return (body.items ?? []).find((one) => (one.body ?? '').includes(text)) ?? null
    },

    async replyToRemark(toText, text) {
      const parent = await this.findRemark(toText)
      if (!parent) return { accepted: false }
      const response = await send(`/api/topics/${subject.id}/comments`, {
        method: 'POST',
        body: JSON.stringify({ body: text, parent_id: parent.id }),
      })
      return { accepted: response.status >= 200 && response.status < 300 }
    },

    async whatItAnswers(text) {
      const reply = await this.findRemark(text)
      return { answers: Boolean(reply && reply.parent_id) }
    },

    async changeRemark(fromText, toText) {
      const remark = await this.findRemark(fromText)
      if (!remark) return { accepted: false }
      const response = await send(`/api/topics/${subject.id}/comments/${remark.id}`, {
        method: 'PATCH',
        body: JSON.stringify({ body: toText }),
      })
      return { accepted: response.status >= 200 && response.status < 300 }
    },

    async removeRemark(text) {
      const remark = await this.findRemark(text)
      if (!remark) return { accepted: false }
      const response = await send(`/api/topics/${subject.id}/comments/${remark.id}/delete`, {
        method: 'POST',
        body: JSON.stringify({ reason: 'no longer wanted' }),
      })
      return { accepted: response.status >= 200 && response.status < 300 }
    },

    async startSubject() {
      const where = (await this.sections())[0]
      const title = `A newly started subject ${Date.now().toString(36)}`
      const made = await send(`/api/sections/${where}/topics`, {
        method: 'POST',
        body: JSON.stringify({ title, body: 'Started so that its place can be seen.', tags: [] }),
      })
      if (made.status >= 300) throw new Error('the board would not take a new subject')
      const topic = await made.json()
      if (topic?.pending) {
        await send(`/api/topics/${topic.id}/commit`, { method: 'POST' })
      }
      return title
    },

    async firstSubjectInSection() {
      const where = (await this.sections())[0]
      const response = await send(`/api/sections/${where}/topics`)
      if (response.status !== 200) return ''
      const body = await response.json()
      return (body.items ?? [])[0]?.title ?? ''
    },
  }
}
