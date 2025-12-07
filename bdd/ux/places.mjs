function required(name) {
  const value = process.env[name]
  if (!value) throw new Error(`set ${name} before running these`)
  return value
}

export function placesFor(which) {
  if (which === 'reference') {
    const base = required('REFERENCE_URL')
    return {
      name: 'reference',
      base,
      places: [
        { what: 'the front page', at: '/' },
        { what: 'a section', at: required('REFERENCE_SECTION') },
        { what: 'the sign-in page', at: required('REFERENCE_SIGN_IN') },
        { what: 'a profile', at: required('REFERENCE_PROFILE') },
        { what: 'an address that leads nowhere', at: '/nothing-answers-to-this-xyzzy' },
      ],
    }
  }
  const base = process.env.CLONE_URL_WEB ?? 'http://127.0.0.1:58082'
  return {
    name: 'this board',
    base,
    places: [
      { what: 'the front page', at: '/' },
      { what: 'a section', at: '/s/general' },
      { what: 'the sign-in page', at: '/sign-in' },
      { what: 'a profile', at: '/u/admin' },
      { what: 'an address that leads nowhere', at: '/nothing-answers-to-this-xyzzy' },
    ],
  }
}
