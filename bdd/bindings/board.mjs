const bindings = {
  clone: () => import('./backend.mjs'),
  reference: () => import('./reference.mjs'),
}

export async function openBoard() {
  const which = process.env.TARGET ?? 'clone'
  const load = bindings[which]
  if (!load) {
    throw new Error(`no binding called "${which}"; try ${Object.keys(bindings).join(' or ')}`)
  }
  const module = await load()
  return module.board()
}

export function cookieJar() {
  const jar = new Map()
  return {
    take(response) {
      const raw = response.headers.getSetCookie?.() ?? []
      for (const line of raw) {
        const [pair] = line.split(';')
        const at = pair.indexOf('=')
        if (at > 0) jar.set(pair.slice(0, at).trim(), pair.slice(at + 1).trim())
      }
    },
    header() {
      return [...jar.entries()].map(([name, value]) => `${name}=${value}`).join('; ')
    },
    forget() {
      jar.clear()
    },
  }
}
