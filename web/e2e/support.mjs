const apiUrl = process.env.API_URL ?? 'http://127.0.0.1:58080'
const rootUser = process.env.E2E_ROOT_USER ?? 'e2e_root'
const rootPass = process.env.E2E_ROOT_PASS ?? 'correcthorse'

export async function promoteViaRoot(username) {
  const signedIn = await fetch(`${apiUrl}/api/sign-in`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: rootUser, password: rootPass }),
  })
  if (!signedIn.ok) {
    throw new Error(`root moderator sign-in failed with ${signedIn.status}`)
  }
  const cookie = signedIn.headers
    .getSetCookie()
    .map((c) => c.split(';')[0])
    .join('; ')
  const promoted = await fetch(
    `${apiUrl}/api/users/${encodeURIComponent(username)}/promote`,
    { method: 'POST', headers: { cookie } },
  )
  if (!promoted.ok) {
    throw new Error(`promoting ${username} failed with ${promoted.status}`)
  }
}
