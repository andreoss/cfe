import { cookieJar } from './board.mjs'

const base = required('REFERENCE_URL')

const people = {
  administrator: { name: required('REFERENCE_ADMIN'), password: required('REFERENCE_PASSWORD') },
  moderator: { name: required('REFERENCE_MODERATOR'), password: required('REFERENCE_PASSWORD') },
  reader: { name: required('REFERENCE_READER'), password: required('REFERENCE_PASSWORD') },
}

function required(name) {
  const value = process.env[name]
  if (!value) {
    throw new Error(`set ${name} before running these`)
  }
  return value
}

export function board() {
  const jar = cookieJar()
  let subject = null

  const humanProof = process.env.REFERENCE_HUMAN_PROOF ?? ''

  function proven(fields) {
    return humanProof ? { ...fields, 'h-captcha-response': humanProof } : fields
  }

  async function get(path) {
    const response = await fetch(`${base}${path}`, {
      headers: { cookie: jar.header() },
      redirect: 'manual',
    })
    jar.take(response)
    return response
  }

  async function csrf() {
    const response = await get('/login.jsp')
    const body = await response.text()
    const found = /name="csrf" value="([^"]*)"/.exec(body)
    if (!found) throw new Error('the sign-in page carried no token')
    return found[1]
  }

  return {
    name: 'reference',

    async answering() {
      const response = await get('/')
      return response.status === 200
    },

    async signIn(role, password) {
      const who = people[role]
      if (!who) throw new Error(`this board has nobody called a ${role}`)
      const token = await csrf()
      const form = new URLSearchParams(
        proven({
          nick: who.name,
          passwd: password ?? who.password,
          csrf: token,
        }),
      )
      const response = await fetch(`${base}/login_process`, {
        method: 'POST',
        headers: {
          cookie: jar.header(),
          'content-type': 'application/x-www-form-urlencoded',
        },
        body: form,
        redirect: 'manual',
      })
      jar.take(response)
      return who.name
    },

    async signOut() {
      const response = await get('/logout')
      const body = await response.text()
      const found = /name="csrf" value="([^"]*)"/.exec(body)
      if (found) {
        const away = await fetch(`${base}/logout`, {
          method: 'POST',
          headers: {
            cookie: jar.header(),
            'content-type': 'application/x-www-form-urlencoded',
          },
          body: new URLSearchParams({ csrf: found[1] }),
          redirect: 'manual',
        })
        jar.take(away)
      }
      jar.forget()
    },

    async whoAmI() {
      const response = await get('/')
      const body = await response.text()
      const mine = /href="\/people\/([^/"]+)\/settings"/.exec(body)
      return mine ? mine[1] : null
    },

    nameFor(role) {
      return people[role]?.name ?? null
    },

    async sections() {
      return required('REFERENCE_SECTIONS')
        .split(',')
        .map((one) => one.trim())
        .filter((one) => one.length > 0)
    },

    async openSection() {
      const where = (await this.sections())[0]
      const response = await get(where)
      const body = await response.text()
      const titled = /<title>([^<]*)/.exec(body)
      return { found: response.status === 200, names: titled ? titled[1].trim() : '' }
    },

    async ensureSubject() {
      return true
    },

    async openSubject() {
      const where = (await this.sections())[0]
      const listing = await (await get(where)).text()
      const link = new RegExp(`${where}[a-z0-9-]+/\\d+`).exec(listing)
      if (!link) throw new Error('the board offered nothing to read')
      const response = await get(link[0])
      const body = await response.text()
      const titled = /<title>([^<]*)/.exec(body)
      const wrote = /href="\/people\/([^/"]+)\/profile"/.exec(body)
      return {
        found: response.status === 200,
        title: titled ? titled[1].split('—')[0].trim() : '',
        author: wrote ? wrote[1] : '',
      }
    },

    async lookUpAccount(role) {
      const who = people[role]
      const response = await get(`/people/${who.name}/profile`)
      const body = await response.text()
      return { found: response.status === 200, names: body.includes(who.name) ? who.name : '' }
    },

    async lookUpMissingAccount() {
      const response = await get(`/people/nobody-answers-to-this-name/profile`)
      return { found: response.status === 200 }
    },

    async openMissingSubject() {
      const where = (await this.sections())[0]
      const response = await get(`${where}nothing/999999999`)
      return { found: response.status === 200 }
    },

    async subjectToWriteOn() {
      if (subject) return subject.id
      await this.signIn('administrator')
      const token = /CSRF_TOKEN=([^;]+)/.exec(jar.header())
      const form = new URLSearchParams(
        proven({
          group: required('REFERENCE_GROUP'),
          title: `A subject to write on ${Date.now().toString(36)}`,
          msg: 'Put here so that a remark has somewhere to go.',
          tags: required('REFERENCE_TAG'),
          csrf: token ? token[1] : '',
        }),
      )
      const response = await fetch(`${base}/add.jsp`, {
        method: 'POST',
        headers: {
          cookie: jar.header(),
          'content-type': 'application/x-www-form-urlencoded',
        },
        body: form,
        redirect: 'manual',
      })
      jar.take(response)
      const went = response.headers.get('location') ?? ''
      const made = /\/([a-z]+)\/([a-z0-9-]+)\/(\d+)/.exec(went)
      if (!made) {
        const body = await response.text()
        const said = /class="error[^"]*"[^>]*>([^<]*)/.exec(body)
        throw new Error(`the board would not take a new subject: ${said ? said[1] : went}`)
      }
      subject = { path: made[0], id: made[3] }
      await this.signOut()
      return subject.id
    },

    async addRemark(text) {
      if (!subject) await this.subjectToWriteOn()
      await get('/login.jsp')
      const token = /CSRF_TOKEN=([^;]+)/.exec(jar.header())
      const form = new URLSearchParams({
        topic: subject.id,
        msg: text,
        csrf: token ? token[1] : '',
      })
      const response = await fetch(`${base}/add_comment_ajax`, {
        method: 'POST',
        headers: {
          cookie: jar.header(),
          'content-type': 'application/x-www-form-urlencoded',
        },
        body: form,
        redirect: 'manual',
      })
      jar.take(response)
      let answered = {}
      try {
        answered = await response.json()
      } catch {
        answered = {}
      }
      return { accepted: typeof answered.url === 'string' }
    },

    async remarksOn(text) {
      if (!subject) return { present: false, author: '' }
      const body = await (await get(subject.path)).text()
      const at = body.indexOf(text)
      if (at < 0) return { present: false, author: '' }
      const before = body.slice(Math.max(0, at - 4000), at)
      const wrote = [...before.matchAll(/href="\/people\/([^/"]+)\/profile"/g)].pop()
      return { present: true, author: wrote ? wrote[1] : '' }
    },

    async findRemark(text) {
      const body = await (await get(subject.path)).text()
      const at = body.indexOf(text)
      if (at < 0) return null
      const before = body.slice(0, at)
      const marked = [...before.matchAll(/id="comment-(\d+)"/g)].pop()
      return marked ? { id: marked[1] } : null
    },

    async replyToRemark(toText, text) {
      const parent = await this.findRemark(toText)
      if (!parent) return { accepted: false }
      const token = /CSRF_TOKEN=([^;]+)/.exec(jar.header())
      const response = await fetch(`${base}/add_comment_ajax`, {
        method: 'POST',
        headers: {
          cookie: jar.header(),
          'content-type': 'application/x-www-form-urlencoded',
        },
        body: new URLSearchParams({
          topic: subject.id,
          replyto: parent.id,
          msg: text,
          csrf: token ? token[1] : '',
        }),
        redirect: 'manual',
      })
      jar.take(response)
      let answered = {}
      try {
        answered = await response.json()
      } catch {
        answered = {}
      }
      return { accepted: typeof answered.url === 'string' }
    },

    async whatItAnswers(text) {
      const body = await (await get(subject.path)).text()
      const at = body.indexOf(text)
      if (at < 0) return { answers: false }
      const around = body.slice(Math.max(0, at - 3000), at)
      return { answers: /Ответ на|class="[^"]*reply|title="Ответ/i.test(around) }
    },

    async changeRemark(fromText, toText) {
      const remark = await this.findRemark(fromText)
      if (!remark) return { accepted: false }
      const token = /CSRF_TOKEN=([^;]+)/.exec(jar.header())
      const response = await fetch(`${base}/edit_comment`, {
        method: 'POST',
        headers: {
          cookie: jar.header(),
          'content-type': 'application/x-www-form-urlencoded',
        },
        body: new URLSearchParams({
          topic: subject.id,
          original: remark.id,
          msg: toText,
          csrf: token ? token[1] : '',
        }),
        redirect: 'manual',
      })
      jar.take(response)
      const after = await this.remarksOn(toText)
      return { accepted: after.present }
    },

    async removeRemark(text) {
      const remark = await this.findRemark(text)
      if (!remark) return { accepted: false }
      const token = /CSRF_TOKEN=([^;]+)/.exec(jar.header())
      const response = await fetch(`${base}/delete_comment.jsp`, {
        method: 'POST',
        headers: {
          cookie: jar.header(),
          'content-type': 'application/x-www-form-urlencoded',
        },
        body: new URLSearchParams({
          msgid: remark.id,
          reason: 'no longer wanted',
          csrf: token ? token[1] : '',
        }),
        redirect: 'manual',
      })
      jar.take(response)
      const still = await this.remarksOn(text)
      return { accepted: !still.present }
    },

    async startSubject() {
      subject = null
      const title = `A newly started subject ${Date.now().toString(36)}`
      const token = /CSRF_TOKEN=([^;]+)/.exec(jar.header())
      const response = await fetch(`${base}/add.jsp`, {
        method: 'POST',
        headers: {
          cookie: jar.header(),
          'content-type': 'application/x-www-form-urlencoded',
        },
        body: new URLSearchParams({
          group: required('REFERENCE_GROUP'),
          title,
          msg: 'Started so that its place can be seen.',
          tags: required('REFERENCE_TAG'),
          csrf: token ? token[1] : '',
        }),
        redirect: 'manual',
      })
      jar.take(response)
      const went = response.headers.get('location') ?? ''
      const made = /\/([a-z]+)\/([a-z0-9-]+)\/(\d+)/.exec(went)
      if (!made) throw new Error('the board would not start a subject')
      subject = { path: made[0], id: made[3] }
      return title
    },

    async startSubjectCarrying(word) {
      subject = null
      const title = `A subject about ${word}`
      const token = /CSRF_TOKEN=([^;]+)/.exec(jar.header())
      const response = await fetch(`${base}/add.jsp`, {
        method: 'POST',
        headers: {
          cookie: jar.header(),
          'content-type': 'application/x-www-form-urlencoded',
        },
        body: new URLSearchParams(
          proven({
            group: required('REFERENCE_GROUP'),
            title,
            msg: `Written so that ${word} can be looked for.`,
            tags: required('REFERENCE_TAG'),
            csrf: token ? token[1] : '',
          }),
        ),
        redirect: 'manual',
      })
      jar.take(response)
      const went = response.headers.get('location') ?? ''
      const made = /\/([a-z]+)\/([a-z0-9-]+)\/(\d+)/.exec(went)
      if (!made) throw new Error('the board would not start a subject')
      subject = { path: made[0], id: made[3] }
      return title
    },

    async findByWord(word) {
      for (let attempt = 0; attempt < 20; attempt += 1) {
        const page = await (await get(`/search.jsp?q=${encodeURIComponent(word)}`)).text()
        const titles = [...page.matchAll(/<article class="msg">\s*<h1>\s*<a[^>]*>([\s\S]*?)<\/a>/g)].map(
          (m) => m[1].replace(/<[^>]*>/g, '').trim(),
        )
        if (titles.length > 0) return { count: titles.length, titles }
        await new Promise((r) => setTimeout(r, 1500))
      }
      return { count: 0, titles: [] }
    },

    async makeAccount(name) {
      const page = await (await get('/register.jsp')).text()
      const csrfToken = /name="csrf" value="([^"]*)"/.exec(page)?.[1] ?? ''
      const permit = /name="permit" value="([^"]*)"/.exec(page)?.[1] ?? ''
      const password = 'correcthorse1'
      const made = await fetch(`${base}/register.jsp`, {
        method: 'POST',
        headers: { cookie: jar.header(), 'content-type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams(
          proven({
            nick: name,
            email: `${name}@example.com`,
            password,
            password2: password,
            rules: 'okay',
            _rules: 'on',
            permit,
            csrf: csrfToken,
          }),
        ),
        redirect: 'manual',
      })
      jar.take(made)
      if (made.status >= 400) return { accepted: false }

      const inbox = required('MAIL_INBOX_URL')
      let code = ''
      for (let attempt = 0; attempt < 20 && !code; attempt += 1) {
        const listed = await (await fetch(`${inbox}/api/v1/messages?limit=25`)).json()
        for (const message of listed.messages ?? []) {
          const full = await (await fetch(`${inbox}/api/v1/message/${message.ID}`)).json()
          const text = full.Text ?? ''
          if (text.includes(name)) {
            code = /activation=([0-9a-f]+)/.exec(text)?.[1] ?? ''
            if (code) break
          }
        }
        if (!code) await new Promise((r) => setTimeout(r, 1000))
      }
      if (!code) return { accepted: false, why: 'no activation arrived' }

      const form = await (await get(`/activate?nick=${name}&activation=${code}`)).text()
      const activateCsrf = /name="csrf" value="([^"]*)"/.exec(form)?.[1] ?? ''
      const done = await fetch(`${base}/activate`, {
        method: 'POST',
        headers: { cookie: jar.header(), 'content-type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams({
          action: 'activate',
          nick: name,
          activation: code,
          passwd: password,
          csrf: activateCsrf,
        }),
        redirect: 'manual',
      })
      jar.take(done)
      jar.forget()
      return { accepted: done.status < 400 }
    },

    async lookUpByName(name) {
      const response = await get(`/people/${name}/profile`)
      return { found: response.status === 200 }
    },

    async changeSubject(body) {
      const page = await (await get(`/edit.jsp?msgid=${subject.id}`)).text()
      const editCsrf = /name="csrf" value="([^"]*)"/.exec(page)?.[1] ?? ''
      const title = /name="title"[^>]*value="([^"]*)"/.exec(page)?.[1] ?? 'A subject'
      const response = await fetch(`${base}/edit.jsp`, {
        method: 'POST',
        headers: { cookie: jar.header(), 'content-type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams(
          proven({
            msgid: subject.id,
            title,
            msg: body,
            tags: required('REFERENCE_TAG'),
            csrf: editCsrf,
          }),
        ),
        redirect: 'manual',
      })
      jar.take(response)
      const after = await (await get(subject.path)).text()
      return { accepted: after.includes(body) }
    },

    async tagForSubjects() {
      return required('REFERENCE_TAG')
    },

    async startSubjectTagged(tag) {
      subject = null
      const title = `A tagged subject ${Date.now().toString(36)}`
      const token = /CSRF_TOKEN=([^;]+)/.exec(jar.header())
      const response = await fetch(`${base}/add.jsp`, {
        method: 'POST',
        headers: { cookie: jar.header(), 'content-type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams(
          proven({
            group: required('REFERENCE_GROUP'),
            title,
            msg: 'Given a tag so it can be seen.',
            tags: tag,
            csrf: token ? token[1] : '',
          }),
        ),
        redirect: 'manual',
      })
      jar.take(response)
      const went = response.headers.get('location') ?? ''
      const made = /\/([a-z]+)\/([a-z0-9-]+)\/(\d+)/.exec(went)
      if (!made) throw new Error('the board would not start a subject')
      subject = { path: made[0], id: made[3] }
      return title
    },

    async tagsOnSubject() {
      const page = await (await get(subject.path)).text()
      return [...page.matchAll(/rel=tag href="\/tag\/([^"]+)"/g)].map((m) => m[1])
    },

    async remarksPerPage() {
      return Number(process.env.REFERENCE_REMARKS_PER_PAGE ?? '50')
    },

    async remarksOnPage(number) {
      const where = number <= 1 ? subject.path : `${subject.path}/page${number - 1}`
      const page = await (await get(where)).text()
      return [...page.matchAll(/Remark \d+ of many [a-z0-9]+/g)].map((m) => m[0])
    },

    async firstSubjectInSection() {
      const where = required('REFERENCE_GROUP_PATH')
      const listing = await (await get(where)).text()
      const first = /class="tracker-title">\s*<p>\s*([^<]+)/.exec(listing)
      return first ? first[1].trim() : ''
    },
  }
}
