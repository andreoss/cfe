const apiUrl = process.env.API_URL ?? 'http://127.0.0.1:58080'
const rootUser = process.env.E2E_ROOT_USER ?? 'e2e_root'
const rootPass = process.env.E2E_ROOT_PASS ?? 'correcthorse'

async function rootCookie() {
  const signedIn = await fetch(`${apiUrl}/api/sign-in`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: rootUser, password: rootPass }),
  })
  if (!signedIn.ok) {
    throw new Error(`root moderator sign-in failed with ${signedIn.status}`)
  }
  return signedIn.headers
    .getSetCookie()
    .map((c) => c.split(';')[0])
    .join('; ')
}

export async function promoteViaRoot(username) {
  const cookie = await rootCookie()
  const promoted = await fetch(
    `${apiUrl}/api/users/${encodeURIComponent(username)}/promote`,
    { method: 'POST', headers: { cookie } },
  )
  if (!promoted.ok) {
    throw new Error(`promoting ${username} failed with ${promoted.status}`)
  }
}

async function allTopics(cookie) {
  const sectionsResponse = await fetch(`${apiUrl}/api/sections`)
  if (!sectionsResponse.ok) {
    throw new Error(`listing sections failed with ${sectionsResponse.status}`)
  }
  const sections = await sectionsResponse.json()
  const found = []
  for (const section of sections) {
    let number = 1
    for (;;) {
      const listed = await fetch(
        `${apiUrl}/api/sections/${section.slug}/topics?page=${number}&size=100`,
        { headers: { cookie } },
      )
      if (!listed.ok) {
        throw new Error(`listing ${section.slug} failed with ${listed.status}`)
      }
      const page = await listed.json()
      found.push(...page.items)
      if (!page.page.has_next) break
      number += 1
    }
  }
  return found
}

async function commit(cookie, id) {
  const done = await fetch(`${apiUrl}/api/topics/${id}/commit`, {
    method: 'POST',
    headers: { cookie },
  })
  if (!done.ok) {
    throw new Error(`committing ${id} failed with ${done.status}`)
  }
}

export async function publishTopic(title) {
  const cookie = await rootCookie()
  for (let attempt = 0; attempt < 60; attempt += 1) {
    const topic = (await allTopics(cookie)).find((t) => t.title === title)
    if (topic) {
      if (topic.pending) await commit(cookie, topic.id)
      return
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error(`topic "${title}" never reached the moderation queue`)
}

export async function publishPending(expected = 1) {
  const cookie = await rootCookie()
  for (let attempt = 0; attempt < 60; attempt += 1) {
    const pending = (await allTopics(cookie)).filter((t) => t.pending)
    if (pending.length >= expected) {
      for (const topic of pending) await commit(cookie, topic.id)
      return pending.length
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error(`fewer than ${expected} topics reached the moderation queue`)
}
