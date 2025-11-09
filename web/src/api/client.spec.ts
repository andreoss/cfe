import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  register,
  signIn,
  me,
  signOut,
  getProfile,
  updateBio,
  getSections,
  getTopics,
  createTopic,
  getTopic,
  getTopicsByTag,
  getComments,
  postComment,
} from './client'

function rawComment(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: '1',
    topic_id: 't1',
    parent_id: null,
    body: 'Nice topic!',
    author_username: 'alice_01',
    created_at: '2026-09-03T00:00:00Z',
    ...overrides,
  }
}

function rawTopic(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: '1',
    section_slug: 'general',
    title: 'Hello',
    body: 'World',
    tags: ['rust'],
    author_username: 'alice_01',
    created_at: '2026-09-03T00:00:00Z',
    ...overrides,
  }
}

function jsonResponse(ok: boolean, body: unknown) {
  return { ok, text: async () => JSON.stringify(body) } as Response
}

function emptyResponse(ok: boolean) {
  return { ok, text: async () => '' } as Response
}

describe('register', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the created user on success', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    const result = await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01' } })
  })

  it('returns the server error on failure', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'username taken' }))
    const result = await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(result).toEqual({ ok: false, error: 'username taken' })
  })

  it('sends credentials so the session cookie is stored', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ credentials: 'include' }),
    )
  })
})

describe('signIn', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the user on success', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    const result = await signIn('alice_01', 'correcthorse')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01' } })
  })

  it('returns invalid-credentials error on failure', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid credentials' }))
    const result = await signIn('alice_01', 'wrong')
    expect(result).toEqual({ ok: false, error: 'invalid credentials' })
  })
})

describe('me', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the current user when a session is active', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    const result = await me()
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01' } })
  })

  it('returns an error when there is no session', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await me()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('signOut', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('succeeds on an empty response body', async () => {
    vi.mocked(fetch).mockResolvedValue(emptyResponse(true))
    const result = await signOut()
    expect(result).toEqual({ ok: true, value: undefined })
  })
})

describe('getProfile', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the profile on success', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', bio: 'hello' }),
    )
    const result = await getProfile('alice_01')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01', bio: 'hello' } })
  })

  it('url-encodes the username', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'a b', bio: null }))
    await getProfile('a b')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/a%20b'),
      expect.anything(),
    )
  })

  it('returns an error for an unknown user', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'user not found' }))
    const result = await getProfile('ghost')
    expect(result).toEqual({ ok: false, error: 'user not found' })
  })
})

describe('updateBio', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the updated profile on success', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', bio: 'new bio' }),
    )
    const result = await updateBio('new bio')
    expect(result).toEqual({ ok: true, value: { id: '1', username: 'alice_01', bio: 'new bio' } })
  })

  it('sends null for an empty bio', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01', bio: null }))
    await updateBio('')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ body: JSON.stringify({ bio: null }) }),
    )
  })
})

describe('getSections', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the section list', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [{ slug: 'general', title: 'General' }]),
    )
    const result = await getSections()
    expect(result).toEqual({ ok: true, value: [{ slug: 'general', title: 'General' }] })
  })
})

describe('getTopics', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Topic type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, [rawTopic()]))
    const result = await getTopics('general')
    expect(result).toEqual({
      ok: true,
      value: [
        {
          id: '1',
          sectionSlug: 'general',
          title: 'Hello',
          body: 'World',
          tags: ['rust'],
          authorUsername: 'alice_01',
          createdAt: '2026-09-03T00:00:00Z',
        },
      ],
    })
  })

  it('returns an error for an unknown section', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'section not found' }))
    const result = await getTopics('ghost')
    expect(result).toEqual({ ok: false, error: 'section not found' })
  })
})

describe('createTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the created topic mapped to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawTopic()))
    const result = await createTopic('general', 'Hello', 'World', ['rust'])
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.sectionSlug).toBe('general')
  })

  it('sends the tags in the request body', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic()))
    await createTopic('general', 'Hello', 'World', ['rust', 'tips'])
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: JSON.stringify({ title: 'Hello', body: 'World', tags: ['rust', 'tips'] }),
      }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await createTopic('general', 'Hello', 'World', [])
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the topic mapped to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawTopic({ id: '42' })))
    const result = await getTopic('42')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.id).toBe('42')
  })

  it('returns an error for an unknown topic', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'topic not found' }))
    const result = await getTopic('ghost')
    expect(result).toEqual({ ok: false, error: 'topic not found' })
  })
})

describe('getTopicsByTag', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns topics mapped to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, [rawTopic()]))
    const result = await getTopicsByTag('rust')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value[0]?.tags).toEqual(['rust'])
  })

  it('returns an empty list for a tag with no topics', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    const result = await getTopicsByTag('nothing')
    expect(result).toEqual({ ok: true, value: [] })
  })
})

describe('getComments', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Comment type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, [rawComment()]))
    const result = await getComments('t1')
    expect(result).toEqual({
      ok: true,
      value: [
        {
          id: '1',
          topicId: 't1',
          parentId: null,
          body: 'Nice topic!',
          authorUsername: 'alice_01',
          createdAt: '2026-09-03T00:00:00Z',
        },
      ],
    })
  })

  it('returns an error for an unknown topic', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'topic not found' }))
    const result = await getComments('ghost')
    expect(result).toEqual({ ok: false, error: 'topic not found' })
  })
})

describe('postComment', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the created comment mapped to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawComment()))
    const result = await postComment('t1', 'Nice topic!', null)
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.parentId).toBeNull()
  })

  it('sends the parent_id when replying', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawComment({ parent_id: 'c1' })))
    await postComment('t1', 'I agree.', 'c1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: JSON.stringify({ body: 'I agree.', parent_id: 'c1' }),
      }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await postComment('t1', 'Nice topic!', null)
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})
