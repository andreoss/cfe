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
  deleteTopic,
  deleteComment,
  editTopic,
  editComment,
  search,
  getNotifications,
  getUnreadCount,
  markNotificationRead,
  getBookmarks,
  getBookmarkState,
  addBookmark,
  removeBookmark,
  REACTION_KINDS,
  getTopicReactions,
  reactToTopic,
  clearTopicReaction,
  getCommentReactions,
  reactToComment,
  clearCommentReaction,
  getPoll,
  createPoll,
  votePoll,
} from './client'

function rawComment(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: '1',
    topic_id: 't1',
    parent_id: null,
    body: 'Nice topic!',
    author_username: 'alice_01',
    created_at: '2026-09-03T00:00:00Z',
    deleted: false,
    deleted_reason: null,
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
    deleted: false,
    deleted_reason: null,
    ...overrides,
  }
}

function rawNotification(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: '1',
    topic_id: 't1',
    topic_title: 'Getting Started',
    comment_id: 'c1',
    actor_username: 'bob_02',
    created_at: '2026-09-03T00:00:00Z',
    read: false,
    ...overrides,
  }
}

function rawPoll(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: 'p1',
    topic_id: 't1',
    question: 'Which one?',
    options: [
      { id: 'o1', text: 'Alpha', votes: 0 },
      { id: 'o2', text: 'Beta', votes: 2 },
    ],
    mine: null,
    total_votes: 2,
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
          deleted: false,
          deletedReason: null,
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
          deleted: false,
          deletedReason: null,
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

describe('deleteTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the deleted topic with reason', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, rawTopic({ deleted: true, deleted_reason: 'spam' })),
    )
    const result = await deleteTopic('t1', 'spam')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.deleted).toBe(true)
    expect(result.ok && result.value.deletedReason).toBe('spam')
  })

  it('returns an error when not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'moderator role required' }))
    const result = await deleteTopic('t1', 'spam')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })
})

describe('deleteComment', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the deleted comment with reason', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, rawComment({ deleted: true, deleted_reason: 'off-topic' })),
    )
    const result = await deleteComment('t1', 'c1', 'off-topic')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.deleted).toBe(true)
    expect(result.ok && result.value.deletedReason).toBe('off-topic')
  })

  it('returns an error when not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'moderator role required' }))
    const result = await deleteComment('t1', 'c1', 'off-topic')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })
})

describe('editTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends a PATCH with the new content and maps the edited flag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, rawTopic({ title: 'After', body: 'New', edited: true })),
    )
    const result = await editTopic('t1', 'After', 'New', ['rust'])
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.title).toBe('After')
    expect(result.ok && result.value.edited).toBe(true)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1'),
      expect.objectContaining({
        method: 'PATCH',
        body: JSON.stringify({ title: 'After', body: 'New', tags: ['rust'] }),
      }),
    )
  })

  it('returns an error when not the author', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'not the author' }))
    const result = await editTopic('t1', 'After', 'New', [])
    expect(result).toEqual({ ok: false, error: 'not the author' })
  })
})

describe('editComment', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends a PATCH with the new body and maps the edited flag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawComment({ body: 'New', edited: true })))
    const result = await editComment('t1', 'c1', 'New')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.body).toBe('New')
    expect(result.ok && result.value.edited).toBe(true)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments/c1'),
      expect.objectContaining({ method: 'PATCH', body: JSON.stringify({ body: 'New' }) }),
    )
  })

  it('returns an error when removed content is edited', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'removed content cannot be edited' }),
    )
    const result = await editComment('t1', 'c1', 'New')
    expect(result).toEqual({ ok: false, error: 'removed content cannot be edited' })
  })
})

describe('search', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps a mixed result list into topic and comment hits', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        { kind: 'topic', ...rawTopic() },
        { kind: 'comment', ...rawComment() },
      ]),
    )
    const result = await search('adapters')
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.value).toHaveLength(2)
    const [first, second] = result.value
    expect(first?.kind === 'topic' && first.topic.title).toBe('Hello')
    expect(second?.kind === 'comment' && second.comment.body).toBe('Nice topic!')
  })

  it('encodes the query in the request', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await search('ports and adapters')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/search?q=ports%20and%20adapters'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error for an invalid query', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid query' }))
    const result = await search('')
    expect(result).toEqual({ ok: false, error: 'invalid query' })
  })
})

describe('getNotifications', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Notification type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, [rawNotification()]))
    const result = await getNotifications()
    expect(result).toEqual({
      ok: true,
      value: [
        {
          id: '1',
          topicId: 't1',
          topicTitle: 'Getting Started',
          commentId: 'c1',
          actorUsername: 'bob_02',
          createdAt: '2026-09-03T00:00:00Z',
          read: false,
        },
      ],
    })
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getNotifications()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getUnreadCount', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the unread envelope to a plain number', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { unread: 3 }))
    const result = await getUnreadCount()
    expect(result).toEqual({ ok: true, value: 3 })
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getUnreadCount()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('markNotificationRead', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the updated notification mapped to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawNotification({ read: true })))
    const result = await markNotificationRead('1')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.read).toBe(true)
  })

  it('url-encodes the notification id', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawNotification()))
    await markNotificationRead('n/1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/notifications/n%2F1/read'),
      expect.anything(),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await markNotificationRead('1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getBookmarks', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Topic type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, [rawTopic()]))
    const result = await getBookmarks()
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
          deleted: false,
          deletedReason: null,
        },
      ],
    })
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getBookmarks()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getBookmarkState', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the bookmarked envelope to a plain boolean', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { bookmarked: true }))
    const result = await getBookmarkState('t1')
    expect(result).toEqual({ ok: true, value: true })
  })

  it('url-encodes the topic id', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { bookmarked: false }))
    await getBookmarkState('t/1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t%2F1/bookmark'),
      expect.anything(),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getBookmarkState('t1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('addBookmark', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the bookmarked envelope to a plain boolean', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { bookmarked: true }))
    const result = await addBookmark('t1')
    expect(result).toEqual({ ok: true, value: true })
  })

  it('uses POST method', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { bookmarked: true }))
    await addBookmark('t1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await addBookmark('t1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('removeBookmark', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the bookmarked envelope to a plain boolean', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { bookmarked: false }))
    const result = await removeBookmark('t1')
    expect(result).toEqual({ ok: true, value: false })
  })

  it('uses DELETE method', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { bookmarked: false }))
    await removeBookmark('t1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await removeBookmark('t1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('REACTION_KINDS', () => {
  it('lists the four supported kinds in display order', () => {
    expect(REACTION_KINDS).toEqual(['like', 'agree', 'disagree', 'thanks'])
  })
})

describe('getTopicReactions', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the counts and the viewer choice', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { counts: [{ kind: 'like', count: 2 }], mine: 'like' }),
    )
    const result = await getTopicReactions('t1')
    expect(result).toEqual({
      ok: true,
      value: { counts: [{ kind: 'like', count: 2 }], mine: 'like' },
    })
  })

  it('defaults a missing choice to null', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { counts: [] }))
    const result = await getTopicReactions('t1')
    expect(result).toEqual({ ok: true, value: { counts: [], mine: null } })
  })

  it('url-encodes the topic id', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { counts: [], mine: null }))
    await getTopicReactions('t/1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t%2F1/reactions'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error for an unknown topic', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'topic not found' }))
    const result = await getTopicReactions('ghost')
    expect(result).toEqual({ ok: false, error: 'topic not found' })
  })
})

describe('reactToTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the chosen kind and returns the new summary', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { counts: [{ kind: 'agree', count: 1 }], mine: 'agree' }),
    )
    const result = await reactToTopic('t1', 'agree')
    expect(result.ok && result.value.mine).toBe('agree')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/reactions'),
      expect.objectContaining({ method: 'POST', body: JSON.stringify({ kind: 'agree' }) }),
    )
  })

  it('returns an error for an unknown kind', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'unknown reaction' }))
    const result = await reactToTopic('t1', 'shrug')
    expect(result).toEqual({ ok: false, error: 'unknown reaction' })
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await reactToTopic('t1', 'like')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('clearTopicReaction', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends a DELETE and returns the summary without a choice', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { counts: [{ kind: 'like', count: 0 }], mine: null }),
    )
    const result = await clearTopicReaction('t1')
    expect(result.ok && result.value.mine).toBeNull()
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/reactions'),
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await clearTopicReaction('t1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getCommentReactions', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the counts for a comment', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { counts: [{ kind: 'thanks', count: 3 }], mine: null }),
    )
    const result = await getCommentReactions('t1', 'c1')
    expect(result).toEqual({
      ok: true,
      value: { counts: [{ kind: 'thanks', count: 3 }], mine: null },
    })
  })

  it('url-encodes both ids', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { counts: [], mine: null }))
    await getCommentReactions('t/1', 'c/1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t%2F1/comments/c%2F1/reactions'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error for an unknown comment', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'comment not found' }))
    const result = await getCommentReactions('t1', 'ghost')
    expect(result).toEqual({ ok: false, error: 'comment not found' })
  })
})

describe('reactToComment', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the chosen kind for a comment', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { counts: [{ kind: 'like', count: 1 }], mine: 'like' }),
    )
    const result = await reactToComment('t1', 'c1', 'like')
    expect(result.ok && result.value.counts).toEqual([{ kind: 'like', count: 1 }])
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments/c1/reactions'),
      expect.objectContaining({ method: 'POST', body: JSON.stringify({ kind: 'like' }) }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await reactToComment('t1', 'c1', 'like')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('clearCommentReaction', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends a DELETE for the comment', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { counts: [], mine: null }))
    const result = await clearCommentReaction('t1', 'c1')
    expect(result).toEqual({ ok: true, value: { counts: [], mine: null } })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments/c1/reactions'),
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await clearCommentReaction('t1', 'c1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getPoll', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields and nested options to the Poll type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawPoll({ mine: 'o2' })))
    const result = await getPoll('t1')
    expect(result).toEqual({
      ok: true,
      value: {
        id: 'p1',
        topicId: 't1',
        question: 'Which one?',
        options: [
          { id: 'o1', text: 'Alpha', votes: 0 },
          { id: 'o2', text: 'Beta', votes: 2 },
        ],
        mine: 'o2',
        totalVotes: 2,
      },
    })
  })

  it('url-encodes the topic id and uses GET', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawPoll()))
    await getPoll('t/1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t%2F1/poll'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error for a topic without a poll', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'poll not found' }))
    const result = await getPoll('t1')
    expect(result).toEqual({ ok: false, error: 'poll not found' })
  })
})

describe('createPoll', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the question and options and returns the mapped poll', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawPoll({ total_votes: 0 })))
    const result = await createPoll('t1', 'Which one?', ['Alpha', 'Beta'])
    expect(result.ok && result.value.totalVotes).toBe(0)
    expect(result.ok && result.value.topicId).toBe('t1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/poll'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ question: 'Which one?', options: ['Alpha', 'Beta'] }),
      }),
    )
  })

  it('returns an error when not the author', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'not the author' }))
    const result = await createPoll('t1', 'Which one?', ['Alpha', 'Beta'])
    expect(result).toEqual({ ok: false, error: 'not the author' })
  })

  it('returns an error when a poll already exists', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'poll already exists' }))
    const result = await createPoll('t1', 'Which one?', ['Alpha', 'Beta'])
    expect(result).toEqual({ ok: false, error: 'poll already exists' })
  })
})

describe('votePoll', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the chosen option and returns the updated poll', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawPoll({ mine: 'o2' })))
    const result = await votePoll('t1', 'o2')
    expect(result.ok && result.value.mine).toBe('o2')
    expect(result.ok && result.value.options[1]?.votes).toBe(2)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/poll/vote'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ option_id: 'o2' }),
      }),
    )
  })

  it('url-encodes the topic id', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawPoll()))
    await votePoll('t/1', 'o1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t%2F1/poll/vote'),
      expect.anything(),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await votePoll('t1', 'o1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})
