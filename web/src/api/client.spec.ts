import { describe, it, expect, vi, beforeEach } from "vitest"
import {
  toSearchOrder,
  toSearchScope,
  attachImage,
  getTopicImages,
  removeImage,
  topicImageUrl,
  register,
  signIn,
  me,
  signOut,
  endAllSessions,
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
  restoreTopic,
  restoreComment,
  editTopic,
  editComment,
  search,
  getActivity,
  sectionFeedUrl,
  tagFeedUrl,
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
  avatarUrl,
  uploadAvatar,
  deleteAvatar,
  changePassword,
  deregister,
  banUser,
  liftBan,
  warnUser,
  getMyWarnings,
  acknowledgeWarnings,
  getIgnoreState,
  ignoreUser,
  stopIgnoring,
  promoteUser,
  setUserRole,
  requestPasswordReset,
  confirmPasswordReset,
  requestEmailChange,
  confirmEmailChange,
  activateAccount,
  listAddressBlocks,
  blockAddress,
  liftAddressBlock,
  getGroups,
  createGroup,
  commitTopic,
  uncommitTopic,
  moveTopic,
  reportTopic,
  reportComment,
  listReports,
  closeReport,
  listAddressPosts,
  removeAddressPosts,
  publishTopic,
  setSticky,
  setOffFront,
  setResolved,
  getWatchState,
  watchTopic,
  unwatchTopic,
  getWatched,
  getRemark,
  setRemark,
  clearRemark,
  getRemarks,
  getInvitationPolicy,
  issueInvitation,
  getInvitations,
  createSection,
  renameSection,
  setSectionScore,
  renameGroup,
  getArchiveMonths,
  getArchiveMonth,
  getTopicHistory,
  getCommentHistory,
  getTopicDifference,
  getTag,
  describeTag,
  makeSynonym,
  followTag,
  unfollowTag,
  getFollowedTags,
  TOPICS_SCORES,
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

function rawReport(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: 'r1',
    topic_id: 't1',
    comment_id: null,
    reporter_username: 'bob_02',
    kind: 'rule',
    reason: 'off topic',
    created_at: '2026-09-03T00:00:00Z',
    ...overrides,
  }
}

function rawAddressPost(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    username: 'bob_02',
    addr: '203.0.113.7',
    client: 'some-browser/1.0',
    at: '2026-09-03T00:00:00Z',
    ...overrides,
  }
}

function rawPage(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    number: 1,
    size: 25,
    total: 1,
    total_pages: 1,
    has_next: false,
    has_previous: false,
    ...overrides,
  }
}

function pagedBody(items: unknown[], overrides: Partial<Record<string, unknown>> = {}) {
  return { items, page: rawPage({ total: items.length, ...overrides }) }
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

describe('endAllSessions', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts to the end-all path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    await endAllSessions()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/me/sessions/end-all',
      expect.objectContaining({ method: 'POST', credentials: 'include' }),
    )
  })

  it('succeeds on an empty response body', async () => {
    vi.mocked(fetch).mockResolvedValue(emptyResponse(true))
    const result = await endAllSessions()
    expect(result).toEqual({ ok: true, value: undefined })
  })

  it('reports the server error when there is no session', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(401, { error: 'missing session' }))
    const result = await endAllSessions()
    expect(result).toEqual({ ok: false, error: 'missing session', status: 401 })
  })
})

describe('getProfile', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the profile on success', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', bio: 'hello', score: 7, role: 'user' }),
    )
    const result = await getProfile('alice_01')
    expect(result).toEqual({
      ok: true,
      value: { id: '1', username: 'alice_01', bio: 'hello', score: 7, role: 'user' },
    })
  })

  it('carries the corrector role', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, {
        id: '2',
        username: 'bob_02',
        bio: null,
        score: 0,
        role: 'corrector',
      }),
    )
    const result = await getProfile('bob_02')
    expect(result.ok && result.value.role).toBe('corrector')
  })

  it('carries the moderator role', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, {
        id: '3',
        username: 'carol_03',
        bio: null,
        score: 0,
        role: 'moderator',
      }),
    )
    const result = await getProfile('carol_03')
    expect(result.ok && result.value.role).toBe('moderator')
  })

  it('carries a negative score', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', bio: null, score: -10 }),
    )
    const result = await getProfile('alice_01')
    expect(result.ok && result.value.score).toBe(-10)
  })

  it('carries a zero score', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', bio: null, score: 0 }),
    )
    const result = await getProfile('alice_01')
    expect(result.ok && result.value.score).toBe(0)
  })

  it('url-encodes the username', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'a b', bio: null, score: 0 }),
    )
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
      jsonResponse(true, {
        id: '1',
        username: 'alice_01',
        bio: 'new bio',
        score: 3,
        role: 'user',
      }),
    )
    const result = await updateBio('new bio')
    expect(result).toEqual({
      ok: true,
      value: { id: '1', username: 'alice_01', bio: 'new bio', score: 3, role: 'user' },
    })
  })

  it('sends null for an empty bio', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', bio: null, score: 0 }),
    )
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
      jsonResponse(true, [
        { slug: 'general', title: 'General', topics_score: 'unrestricted', may_post: true },
      ]),
    )
    const result = await getSections()
    expect(result).toEqual({
      ok: true,
      value: [{ slug: 'general', title: 'General', topicsScore: 'unrestricted', mayPost: true }],
    })
  })

  it('maps snake_case fields to the Section type', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        { slug: 'staff', title: 'Staff', topics_score: 'moderators-only', may_post: false },
      ]),
    )
    const result = await getSections()
    const [first] = result.ok ? result.value : []
    expect(first?.topicsScore).toBe('moderators-only')
    expect(first?.mayPost).toBe(false)
  })

  it('carries the standing the server computed for each section', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        { slug: 'general', title: 'General', topics_score: 'unrestricted', may_post: true },
        { slug: 'staff', title: 'Staff', topics_score: 'moderators-only', may_post: false },
      ]),
    )
    const result = await getSections()
    expect(result.ok && result.value.map((s) => s.mayPost)).toEqual([true, false])
  })

  it('returns the server error when the listing cannot be read', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'service unavailable' }))
    const result = await getSections()
    expect(result).toEqual({ ok: false, error: 'service unavailable' })
  })
})

describe('getTopics', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Topic type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawTopic()])))
    const result = await getTopics('general')
    expect(result.ok && result.value.items).toEqual([
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
    ])
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawTopic()], {
          number: 2,
          size: 10,
          total: 7,
          total_pages: 3,
          has_next: true,
          has_previous: true,
        }),
      ),
    )
    const result = await getTopics('general', 2, 10)
    expect(result.ok && result.value.page).toEqual({
      number: 2,
      size: 10,
      total: 7,
      totalPages: 3,
      hasNext: true,
      hasPrevious: true,
    })
  })

  it('omits the query string when no page is asked for', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getTopics('general')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.not.stringContaining('?'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('carries the page and size in the query string', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getTopics('general', 3, 50)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/general/topics?page=3&size=50'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('carries the page alone when no size is given', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getTopics('general', 2)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/general/topics?page=2'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an empty page past the end with the real total', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([], { number: 9, total: 7, total_pages: 3, has_previous: true }),
      ),
    )
    const result = await getTopics('general', 9)
    expect(result.ok && result.value.items).toEqual([])
    expect(result.ok && result.value.page.total).toBe(7)
  })

  it('returns the server error for an out-of-range page', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid page' }))
    const result = await getTopics('general', 0)
    expect(result).toEqual({ ok: false, error: 'invalid page' })
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

describe('groups and premoderation', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('createTopic sends the group slug in the body', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic()))
    await createTopic('general', 'Hello', 'World', ['rust'], 'announcements')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: JSON.stringify({
          title: 'Hello',
          body: 'World',
          tags: ['rust'],
          group: 'announcements',
        }),
      }),
    )
  })

  it('maps group_slug and pending from the topic response', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        rawTopic({ id: '7', group_slug: 'announcements', pending: true }),
      ),
    )
    const result = await getTopic('7')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.groupSlug).toBe('announcements')
    expect(result.ok && result.value.pending).toBe(true)
  })

  it('getGroups lists groups in a section', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        { id: 'g1', section_slug: 'general', name: 'Announcements', slug: 'announcements' },
      ]),
    )
    const result = await getGroups('general')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value[0]?.slug).toBe('announcements')
  })

  it('getGroups maps snake_case fields to the Group type', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        { id: 'g1', section_slug: 'general', name: 'Announcements', slug: 'announcements' },
      ]),
    )
    const result = await getGroups('general')
    expect(result.ok && result.value).toEqual([
      { id: 'g1', sectionSlug: 'general', name: 'Announcements', slug: 'announcements' },
    ])
  })

  it('createGroup maps snake_case fields to the Group type', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, {
        id: 'g1',
        section_slug: 'general',
        name: 'Announcements',
        slug: 'announcements',
      }),
    )
    const result = await createGroup('general', 'Announcements', 'announcements')
    expect(result).toEqual({
      ok: true,
      value: { id: 'g1', sectionSlug: 'general', name: 'Announcements', slug: 'announcements' },
    })
  })

  it('createGroup posts the name and slug', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, {
        id: 'g1',
        section_slug: 'general',
        name: 'Announcements',
        slug: 'announcements',
      }),
    )
    await createGroup('general', 'Announcements', 'announcements')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: JSON.stringify({ name: 'Announcements', slug: 'announcements' }),
      }),
    )
  })

  it('commitTopic posts to the commit endpoint', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ id: '7', pending: false })))
    const result = await commitTopic('7')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/7/commit',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(result.ok && result.value.pending).toBe(false)
  })

  it('uncommitTopic posts to the uncommit endpoint', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ id: '7', pending: true })))
    const result = await uncommitTopic('7')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/7/uncommit',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(result.ok && result.value.pending).toBe(true)
  })

  it('moveTopic posts the target group', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ id: '7', group_slug: 'news' })))
    const result = await moveTopic('7', 'news')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/7/move',
      expect.objectContaining({ body: JSON.stringify({ group: 'news' }) }),
    )
    expect(result.ok && result.value.groupSlug).toBe('news')
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
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawTopic()])))
    const result = await getTopicsByTag('rust')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.items[0]?.tags).toEqual(['rust'])
  })

  it('returns an empty list for a tag with no topics', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([], { total_pages: 0 })))
    const result = await getTopicsByTag('nothing')
    expect(result.ok && result.value.items).toEqual([])
    expect(result.ok && result.value.page.totalPages).toBe(0)
  })

  it('carries the page and size in the query string', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getTopicsByTag('rust', 2, 5)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/tags/rust/topics?page=2&size=5'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns the server error for an out-of-range page', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid page' }))
    const result = await getTopicsByTag('rust', 999999)
    expect(result).toEqual({ ok: false, error: 'invalid page' })
  })
})

describe('getComments', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Comment type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawComment()])))
    const result = await getComments('t1')
    expect(result.ok && result.value.items).toEqual([
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
    ])
  })

  it('maps the ignored flag of a hidden comment', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, pagedBody([rawComment({ body: '', ignored: true })])),
    )
    const result = await getComments('t1')
    expect(result.ok).toBe(true)
    if (!result.ok) return
    const [first] = result.value.items
    expect(first?.ignored).toBe(true)
    expect(first?.body).toBe('')
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawComment()], {
          number: 1,
          size: 25,
          total: 30,
          total_pages: 2,
          has_next: true,
        }),
      ),
    )
    const result = await getComments('t1')
    expect(result.ok && result.value.page).toEqual({
      number: 1,
      size: 25,
      total: 30,
      totalPages: 2,
      hasNext: true,
      hasPrevious: false,
    })
  })

  it('carries the page in the query string after the encoded id', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getComments('t/1', 2)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t%2F1/comments?page=2'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns the server error for an out-of-range size', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid page' }))
    const result = await getComments('t1', 1, 500)
    expect(result).toEqual({ ok: false, error: 'invalid page' })
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

  it('sends only the reason when no penalty is chosen', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ deleted: true })))
    await deleteTopic('t1', 'spam')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/delete'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ reason: 'spam' }),
      }),
    )
  })

  it('sends the penalty when one is chosen', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ deleted: true })))
    await deleteTopic('t1', 'spam', -50)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/delete'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ reason: 'spam', penalty: -50 }),
      }),
    )
  })

  it('sends a zero penalty rather than omitting it', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ deleted: true })))
    await deleteTopic('t1', 'spam', 0)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ body: JSON.stringify({ reason: 'spam', penalty: 0 }) }),
    )
  })

  it('returns an error when not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'moderator role required' }))
    const result = await deleteTopic('t1', 'spam')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })

  it('returns the out of range error verbatim', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'penalty out of range' }))
    const result = await deleteTopic('t1', 'spam', -60)
    expect(result).toEqual({ ok: false, error: 'penalty out of range' })
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

  it('sends only the reason when no penalty is chosen', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawComment({ deleted: true })))
    await deleteComment('t1', 'c1', 'off-topic')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments/c1/delete'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ reason: 'off-topic' }),
      }),
    )
  })

  it('sends the penalty when one is chosen', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawComment({ deleted: true })))
    await deleteComment('t1', 'c1', 'off-topic', -20)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments/c1/delete'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ reason: 'off-topic', penalty: -20 }),
      }),
    )
  })

  it('sends a zero penalty rather than omitting it', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawComment({ deleted: true })))
    await deleteComment('t1', 'c1', 'off-topic', 0)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ body: JSON.stringify({ reason: 'off-topic', penalty: 0 }) }),
    )
  })

  it('returns an error when not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'moderator role required' }))
    const result = await deleteComment('t1', 'c1', 'off-topic')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })

  it('returns the out of range error verbatim', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'penalty out of range' }))
    const result = await deleteComment('t1', 'c1', 'off-topic', 5)
    expect(result).toEqual({ ok: false, error: 'penalty out of range' })
  })
})

describe('restoreTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the restored topic without a deletion reason', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, rawTopic({ deleted: false, deleted_reason: null })),
    )
    const result = await restoreTopic('t1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/restore'),
      expect.objectContaining({ method: 'POST' }),
    )
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.deleted).toBe(false)
    expect(result.ok && result.value.deletedReason).toBe(null)
  })

  it('returns the not deleted error verbatim', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'not deleted' }))
    const result = await restoreTopic('t1')
    expect(result).toEqual({ ok: false, error: 'not deleted' })
  })
})

describe('restoreComment', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the restored comment without a deletion reason', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, rawComment({ deleted: false, deleted_reason: null })),
    )
    const result = await restoreComment('t1', 'c1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments/c1/restore'),
      expect.objectContaining({ method: 'POST' }),
    )
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.deleted).toBe(false)
    expect(result.ok && result.value.deletedReason).toBe(null)
  })

  it('returns the not deleted error verbatim', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'not deleted' }))
    const result = await restoreComment('t1', 'c1')
    expect(result).toEqual({ ok: false, error: 'not deleted' })
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

describe('getActivity', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps a mixed activity list into topic and comment hits', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        { kind: 'topic', ...rawTopic() },
        { kind: 'comment', ...rawComment() },
      ]),
    )
    const result = await getActivity()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.value).toHaveLength(2)
    const [first, second] = result.value
    expect(first?.kind === 'topic' && first.topic.sectionSlug).toBe('general')
    expect(first?.kind === 'topic' && first.topic.authorUsername).toBe('alice_01')
    expect(second?.kind === 'comment' && second.comment.topicId).toBe('t1')
    expect(second?.kind === 'comment' && second.comment.authorUsername).toBe('alice_01')
  })

  it('requests the activity path with GET', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getActivity()
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/activity'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an empty list when there is no activity', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    const result = await getActivity()
    expect(result).toEqual({ ok: true, value: [] })
  })

  it('returns an error when the request fails', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'service unavailable' }))
    const result = await getActivity()
    expect(result).toEqual({ ok: false, error: 'service unavailable' })
  })
})

describe('getNotifications', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Notification type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawNotification()])))
    const result = await getNotifications()
    expect(result.ok && result.value.items).toEqual([
      {
        id: '1',
        topicId: 't1',
        topicTitle: 'Getting Started',
        commentId: 'c1',
        actorUsername: 'bob_02',
        createdAt: '2026-09-03T00:00:00Z',
        read: false,
      },
    ])
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawNotification()], {
          number: 3,
          size: 25,
          total: 60,
          total_pages: 3,
          has_previous: true,
        }),
      ),
    )
    const result = await getNotifications(3)
    expect(result.ok && result.value.page).toEqual({
      number: 3,
      size: 25,
      total: 60,
      totalPages: 3,
      hasNext: false,
      hasPrevious: true,
    })
  })

  it('carries the page and size in the query string', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getNotifications(2, 25)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/notifications?page=2&size=25'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns the server error for an out-of-range page', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid page' }))
    const result = await getNotifications(-1)
    expect(result).toEqual({ ok: false, error: 'invalid page' })
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
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawTopic()])))
    const result = await getBookmarks()
    expect(result.ok && result.value.items).toEqual([
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
    ])
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawTopic()], {
          number: 2,
          size: 25,
          total: 51,
          total_pages: 3,
          has_next: true,
          has_previous: true,
        }),
      ),
    )
    const result = await getBookmarks(2)
    expect(result.ok && result.value.page).toEqual({
      number: 2,
      size: 25,
      total: 51,
      totalPages: 3,
      hasNext: true,
      hasPrevious: true,
    })
  })

  it('carries the page and size in the query string', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getBookmarks(4, 100)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/bookmarks?page=4&size=100'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns the server error for an out-of-range page', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid page' }))
    const result = await getBookmarks(0)
    expect(result).toEqual({ ok: false, error: 'invalid page' })
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

describe('feed urls', () => {
  it('builds a section feed url', () => {
    expect(sectionFeedUrl('general')).toContain('/api/sections/general/feed')
  })

  it('builds a tag feed url', () => {
    expect(tagFeedUrl('news')).toContain('/api/tags/news/feed')
  })

  it('encodes an awkward slug', () => {
    expect(sectionFeedUrl('a b')).toContain('/api/sections/a%20b/feed')
  })
})

describe('avatarUrl', () => {
  it('builds an avatar url', () => {
    expect(avatarUrl('alice_01')).toContain('/api/users/alice_01/avatar')
  })

  it('encodes an awkward username', () => {
    expect(avatarUrl('a b/c')).toContain('/api/users/a%20b%2Fc/avatar')
  })
})

describe('uploadAvatar', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the has_avatar envelope to a plain boolean', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { has_avatar: true }))
    const result = await uploadAvatar('QUJD')
    expect(result).toEqual({ ok: true, value: true })
  })

  it('uses POST method', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { has_avatar: true }))
    await uploadAvatar('QUJD')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/me/avatar'),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('sends the base64 payload as data', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { has_avatar: true }))
    await uploadAvatar('QUJD')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ body: JSON.stringify({ data: 'QUJD' }) }),
    )
  })

  it('returns the server error for an unsupported image', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'unsupported image' }))
    const result = await uploadAvatar('QUJD')
    expect(result).toEqual({ ok: false, error: 'unsupported image' })
  })
})

describe('deleteAvatar', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the has_avatar envelope to a plain boolean', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { has_avatar: false }))
    const result = await deleteAvatar()
    expect(result).toEqual({ ok: true, value: false })
  })

  it('uses DELETE method', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { has_avatar: false }))
    await deleteAvatar()
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/me/avatar'),
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await deleteAvatar()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('changePassword', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the user on success', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'member' }),
    )
    const result = await changePassword('correcthorse', 'batterystaple')
    expect(result).toEqual({
      ok: true,
      value: { id: '1', username: 'alice_01', role: 'member' },
    })
  })

  it('uses POST method', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'member' }),
    )
    await changePassword('correcthorse', 'batterystaple')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/me/password'),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('sends the passwords under snake_case keys', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'member' }),
    )
    await changePassword('correcthorse', 'batterystaple')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: JSON.stringify({
          current_password: 'correcthorse',
          new_password: 'batterystaple',
        }),
      }),
    )
  })

  it('returns the server error for a wrong current password', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'wrong current password' }),
    )
    const result = await changePassword('nope', 'batterystaple')
    expect(result).toEqual({ ok: false, error: 'wrong current password' })
  })

  it('returns the server error for an invalid new password', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'invalid new password' }),
    )
    const result = await changePassword('correcthorse', 'short')
    expect(result).toEqual({ ok: false, error: 'invalid new password' })
  })
})

describe('deregister', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the user on success', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'member' }),
    )
    const result = await deregister()
    expect(result).toEqual({
      ok: true,
      value: { id: '1', username: 'alice_01', role: 'member' },
    })
  })

  it('uses POST method', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'member' }),
    )
    await deregister()
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/me/deregister'),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await deregister()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

function rawWarning(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: 'w1',
    reason: 'Please stay on topic',
    created_at: '2026-09-03T00:00:00Z',
    acknowledged: false,
    ...overrides,
  }
}

describe('banUser', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the reason and days and returns the ban', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { reason: 'spam', until: '2026-09-10T00:00:00Z' }),
    )
    const result = await banUser('bob_02', 'spam', 7)
    expect(result).toEqual({
      ok: true,
      value: { reason: 'spam', until: '2026-09-10T00:00:00Z' },
    })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob_02/ban'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ reason: 'spam', days: 7 }),
      }),
    )
  })

  it('sends a null days for a permanent ban', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { reason: 'spam', until: null }))
    await banUser('bob_02', 'spam', null)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ body: JSON.stringify({ reason: 'spam', days: null }) }),
    )
  })

  it('url-encodes the username', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { reason: 'spam', until: null }))
    await banUser('bob 02/x', 'spam', null)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob%2002%2Fx/ban'),
      expect.anything(),
    )
  })

  it('returns the server error for a plain user', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'moderator role required' }),
    )
    const result = await banUser('bob_02', 'spam', null)
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })

  it('returns the server error for banning yourself', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'cannot ban yourself' }))
    const result = await banUser('alice_01', 'spam', null)
    expect(result).toEqual({ ok: false, error: 'cannot ban yourself' })
  })
})

describe('liftBan', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends a DELETE and succeeds on an empty body', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await liftBan('bob_02')
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob_02/ban'),
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('returns the server error for a plain user', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'moderator role required' }),
    )
    const result = await liftBan('bob_02')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })
})

describe('warnUser', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Warning type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawWarning()))
    const result = await warnUser('bob_02', 'Please stay on topic')
    expect(result).toEqual({
      ok: true,
      value: {
        id: 'w1',
        reason: 'Please stay on topic',
        createdAt: '2026-09-03T00:00:00Z',
        acknowledged: false,
      },
    })
  })

  it('posts the reason to the warn path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawWarning()))
    await warnUser('bob 02', 'Please stay on topic')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob%2002/warn'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ reason: 'Please stay on topic' }),
      }),
    )
  })

  it('returns the server error for a plain user', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'moderator role required' }),
    )
    const result = await warnUser('bob_02', 'Please stay on topic')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })

  it('returns the server error for an unknown user', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'user not found' }))
    const result = await warnUser('ghost', 'Please stay on topic')
    expect(result).toEqual({ ok: false, error: 'user not found' })
  })
})

describe('getMyWarnings', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps every warning to camelCase', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, [rawWarning(), rawWarning({ id: 'w2', acknowledged: true })]),
    )
    const result = await getMyWarnings()
    expect(result.ok && result.value).toEqual([
      {
        id: 'w1',
        reason: 'Please stay on topic',
        createdAt: '2026-09-03T00:00:00Z',
        acknowledged: false,
      },
      {
        id: 'w2',
        reason: 'Please stay on topic',
        createdAt: '2026-09-03T00:00:00Z',
        acknowledged: true,
      },
    ])
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/me/warnings'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an empty list when there are none', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    const result = await getMyWarnings()
    expect(result).toEqual({ ok: true, value: [] })
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getMyWarnings()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('acknowledgeWarnings', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts to the acknowledge path and succeeds on an empty body', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await acknowledgeWarnings()
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/me/warnings/acknowledge'),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await acknowledgeWarnings()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getIgnoreState', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the ignored envelope to a plain boolean', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { ignored: true }))
    const result = await getIgnoreState('bob_02')
    expect(result).toEqual({ ok: true, value: true })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob_02/ignore'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getIgnoreState('bob_02')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('ignoreUser', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts and unwraps the ignored envelope', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { ignored: true }))
    const result = await ignoreUser('bob_02')
    expect(result).toEqual({ ok: true, value: true })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob_02/ignore'),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('url-encodes the username', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { ignored: true }))
    await ignoreUser('bob 02/x')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob%2002%2Fx/ignore'),
      expect.anything(),
    )
  })

  it('returns the server error for ignoring yourself', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'cannot ignore yourself' }),
    )
    const result = await ignoreUser('alice_01')
    expect(result).toEqual({ ok: false, error: 'cannot ignore yourself' })
  })
})

describe('stopIgnoring', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends a DELETE and unwraps the ignored envelope', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { ignored: false }))
    const result = await stopIgnoring('bob_02')
    expect(result).toEqual({ ok: true, value: false })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob_02/ignore'),
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await stopIgnoring('bob_02')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('promoteUser', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts to the promote path and returns the promoted user', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'moderator' }),
    )
    const result = await promoteUser('alice_01')
    expect(result.ok && result.value.role).toBe('moderator')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/alice_01/promote'),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('returns an error when not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'moderator role required' }),
    )
    const result = await promoteUser('alice_01')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })
})

describe('setUserRole', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the role to the role path and returns the updated user', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'corrector' }),
    )
    const result = await setUserRole('alice_01', 'corrector')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/users/alice_01/role',
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role: 'corrector' }),
      }),
    )
    expect(result).toEqual({
      ok: true,
      value: { id: '1', username: 'alice_01', role: 'corrector' },
    })
  })

  it('sends the moderator role and encodes the username in the path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '2', username: 'bob 02', role: 'moderator' }),
    )
    await setUserRole('bob 02', 'moderator')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/users/bob%2002/role',
      expect.objectContaining({ body: JSON.stringify({ role: 'moderator' }) }),
    )
  })

  it('reports the error when the caller is not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(403, { error: 'moderator role required' }),
    )
    const result = await setUserRole('alice_01', 'moderator')
    expect(result).toEqual({ ok: false, error: 'moderator role required', status: 403 })
  })

  it('reports the error when the role is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(422, { error: 'invalid role' }))
    const result = await setUserRole('alice_01', 'user')
    expect(result).toEqual({ ok: false, error: 'invalid role', status: 422 })
  })
})

describe('listAddressBlocks', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps raw blocks to camelCase', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, [
        { addr: '203.0.113.7', reason: 'spam', blocked_at: '2026-09-03T00:00:00Z', until: null },
      ]),
    )
    const result = await listAddressBlocks()
    expect(result.ok && result.value[0]).toEqual({
      addr: '203.0.113.7',
      reason: 'spam',
      blockedAt: '2026-09-03T00:00:00Z',
      until: null,
    })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/address-blocks'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error when not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'moderator role required' }),
    )
    const result = await listAddressBlocks()
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })
})

describe('blockAddress', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the block and returns the mapped block', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, {
        addr: '203.0.113.7',
        reason: 'spam',
        blocked_at: '2026-09-03T00:00:00Z',
        until: '2026-09-10T00:00:00Z',
      }),
    )
    const result = await blockAddress('203.0.113.7', 'spam', 7)
    expect(result.ok && result.value).toEqual({
      addr: '203.0.113.7',
      reason: 'spam',
      blockedAt: '2026-09-03T00:00:00Z',
      until: '2026-09-10T00:00:00Z',
    })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/address-blocks'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ addr: '203.0.113.7', reason: 'spam', days: 7 }),
      }),
    )
  })

  it('sends a null days when the block is for good', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, {
        addr: '203.0.113.7',
        reason: 'spam',
        blocked_at: '2026-09-03T00:00:00Z',
        until: null,
      }),
    )
    await blockAddress('203.0.113.7', 'spam', null)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/address-blocks'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ addr: '203.0.113.7', reason: 'spam', days: null }),
      }),
    )
  })

  it('returns an error on a non-ok response', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid address' }))
    const result = await blockAddress('nope', 'spam', null)
    expect(result).toEqual({ ok: false, error: 'invalid address' })
  })
})

describe('liftAddressBlock', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('deletes the block and returns void', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await liftAddressBlock('203.0.113.7')
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/address-blocks/203.0.113.7'),
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('returns an error on failure', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'not found' }))
    const result = await liftAddressBlock('203.0.113.7')
    expect(result).toEqual({ ok: false, error: 'not found' })
  })
})

describe('requestPasswordReset', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the address and succeeds on an empty body', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await requestPasswordReset('alice@example.com')
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/password-reset'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ email: 'alice@example.com' }),
      }),
    )
  })

  it('succeeds the same way for an unknown address', async () => {
    vi.mocked(fetch).mockResolvedValue(emptyResponse(true))
    const result = await requestPasswordReset('nobody@example.com')
    expect(result).toEqual({ ok: true, value: undefined })
  })

  it('returns the server error for an invalid address', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid email' }))
    const result = await requestPasswordReset('nope')
    expect(result).toEqual({ ok: false, error: 'invalid email' })
  })
})

describe('confirmPasswordReset', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends the code and the new password under snake_case keys', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await confirmPasswordReset('a'.repeat(64), 'batterystaple')
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/password-reset/confirm'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ code: 'a'.repeat(64), new_password: 'batterystaple' }),
      }),
    )
  })

  it('returns the server error for an expired code', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'invalid or expired code' }),
    )
    const result = await confirmPasswordReset('b'.repeat(64), 'batterystaple')
    expect(result).toEqual({ ok: false, error: 'invalid or expired code' })
  })

  it('returns the server error for a short new password', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'invalid new password' }),
    )
    const result = await confirmPasswordReset('a'.repeat(64), 'short')
    expect(result).toEqual({ ok: false, error: 'invalid new password' })
  })
})

describe('requestEmailChange', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the new address and succeeds on an empty body', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await requestEmailChange('alice@example.org')
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/me/email'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ email: 'alice@example.org' }),
      }),
    )
  })

  it('returns the server error for an address already taken', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'address taken' }))
    const result = await requestEmailChange('bob@example.org')
    expect(result).toEqual({ ok: false, error: 'address taken' })
  })

  it('returns the server error for an invalid address', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'invalid email' }))
    const result = await requestEmailChange('nope')
    expect(result).toEqual({ ok: false, error: 'invalid email' })
  })
})

describe('confirmEmailChange', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the code and returns the user', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'member' }),
    )
    const result = await confirmEmailChange('c'.repeat(64))
    expect(result).toEqual({
      ok: true,
      value: { id: '1', username: 'alice_01', role: 'member' },
    })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/me/email/confirm'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ code: 'c'.repeat(64) }),
      }),
    )
  })

  it('returns the server error for a bad code', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'invalid or expired code' }),
    )
    const result = await confirmEmailChange('d'.repeat(64))
    expect(result).toEqual({ ok: false, error: 'invalid or expired code' })
  })
})

describe('activateAccount', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the code and returns the user', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, { id: '1', username: 'alice_01', role: 'member' }),
    )
    const result = await activateAccount('e'.repeat(64))
    expect(result).toEqual({
      ok: true,
      value: { id: '1', username: 'alice_01', role: 'member' },
    })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/activate'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ code: 'e'.repeat(64) }),
      }),
    )
  })

  it('returns the server error for a bad code', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'invalid or expired code' }),
    )
    const result = await activateAccount('f'.repeat(64))
    expect(result).toEqual({ ok: false, error: 'invalid or expired code' })
  })
})

describe('reportTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the kind and reason to the topic report path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await reportTopic('t1', 'rule', 'off topic')
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/report'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ kind: 'rule', reason: 'off topic' }),
      }),
    )
  })

  it('url-encodes the topic id', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    await reportTopic('t/1', 'tag', 'wrong tag')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t%2F1/report'),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('returns the server error when the topic was already reported', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'already reported' }))
    const result = await reportTopic('t1', 'rule', 'off topic')
    expect(result).toEqual({ ok: false, error: 'already reported' })
  })

  it('returns the server error when the topic is missing', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'topic not found' }))
    const result = await reportTopic('t9', 'rule', 'off topic')
    expect(result).toEqual({ ok: false, error: 'topic not found' })
  })
})

describe('reportComment', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the kind and reason to the comment report path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await reportComment('t1', 'c1', 'spelling', 'typo in the title')
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments/c1/report'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ kind: 'spelling', reason: 'typo in the title' }),
      }),
    )
  })

  it('returns the server error when the comment was already reported', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'already reported' }))
    const result = await reportComment('t1', 'c1', 'group', 'wrong group')
    expect(result).toEqual({ ok: false, error: 'already reported' })
  })
})

describe('listReports', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Report type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawReport()])))
    const result = await listReports()
    expect(result.ok && result.value.items).toEqual([
      {
        id: 'r1',
        topicId: 't1',
        commentId: null,
        reporterUsername: 'bob_02',
        kind: 'rule',
        reason: 'off topic',
        createdAt: '2026-09-03T00:00:00Z',
      },
    ])
  })

  it('keeps the comment id of a comment report', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, pagedBody([rawReport({ comment_id: 'c1', kind: 'tag' })])),
    )
    const result = await listReports()
    expect(result.ok && result.value.items[0]?.commentId).toEqual('c1')
    expect(result.ok && result.value.items[0]?.kind).toEqual('tag')
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawReport()], {
          number: 2,
          size: 10,
          total: 7,
          total_pages: 3,
          has_next: true,
          has_previous: true,
        }),
      ),
    )
    const result = await listReports(2, 10)
    expect(result.ok && result.value.page).toEqual({
      number: 2,
      size: 10,
      total: 7,
      totalPages: 3,
      hasNext: true,
      hasPrevious: true,
    })
  })

  it('carries the page and size in the query string', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await listReports(2, 25)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/reports?page=2&size=25'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns the server error for a non-moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'moderator role required' }),
    )
    const result = await listReports()
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })
})

describe('closeReport', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts to the close path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await closeReport('r1')
    expect(result).toEqual({ ok: true, value: undefined })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/reports/r1/close'),
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('returns the server error when the report is already closed', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'already closed' }))
    const result = await closeReport('r1')
    expect(result).toEqual({ ok: false, error: 'already closed' })
  })
})

describe('listAddressPosts', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('encodes the address and carries the page and size in the query string', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await listAddressPosts('2001:db8::1', 2, 25)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/addresses/2001%3Adb8%3A%3A1/posts?page=2&size=25'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('maps an item with no client', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, pagedBody([rawAddressPost({ username: 'unknown', client: null })])),
    )
    const result = await listAddressPosts('203.0.113.7')
    expect(result.ok && result.value.items).toEqual([
      {
        username: 'unknown',
        addr: '203.0.113.7',
        client: null,
        at: '2026-09-03T00:00:00Z',
      },
    ])
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawAddressPost()], {
          number: 2,
          size: 10,
          total: 7,
          total_pages: 3,
          has_next: true,
          has_previous: true,
        }),
      ),
    )
    const result = await listAddressPosts('203.0.113.7', 2, 10)
    expect(result.ok && result.value.page).toEqual({
      number: 2,
      size: 10,
      total: 7,
      totalPages: 3,
      hasNext: true,
      hasPrevious: true,
    })
  })

  it('returns the server error for a non-moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'moderator role required' }),
    )
    const result = await listAddressPosts('203.0.113.7')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })
})

describe('removeAddressPosts', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the hours and the reason to the removal path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { removed: 4 }))
    const result = await removeAddressPosts('203.0.113.7', 12, 'flooding')
    expect(result).toEqual({ ok: true, value: { removed: 4 } })
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/addresses/203.0.113.7/remove-posts'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ hours: 12, reason: 'flooding' }),
      }),
    )
  })

  it('returns the server error for a non-moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(false, { error: 'moderator role required' }),
    )
    const result = await removeAddressPosts('203.0.113.7', 12, 'flooding')
    expect(result).toEqual({ ok: false, error: 'moderator role required' })
  })
})

describe('open report counts', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps open_reports on a topic', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawTopic({ open_reports: 3 })))
    const result = await getTopic('1')
    expect(result.ok && result.value.openReports).toEqual(3)
  })
})

function statusResponse(status: number, body: unknown) {
  return { ok: false, status, text: async () => JSON.stringify(body) } as Response
}

describe('challenge', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('register sends the challenge when one is supplied', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    await register('alice_01', 'alice@example.com', 'correcthorse', 'a blue moon')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/register'),
      expect.objectContaining({
        body: JSON.stringify({
          username: 'alice_01',
          email: 'alice@example.com',
          password: 'correcthorse',
          challenge: 'a blue moon',
        }),
      }),
    )
  })

  it('register omits the challenge when none is supplied', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/register'),
      expect.objectContaining({
        body: JSON.stringify({
          username: 'alice_01',
          email: 'alice@example.com',
          password: 'correcthorse',
        }),
      }),
    )
  })

  it('register omits the challenge when it is empty', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    await register('alice_01', 'alice@example.com', 'correcthorse', '')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/register'),
      expect.objectContaining({
        body: JSON.stringify({
          username: 'alice_01',
          email: 'alice@example.com',
          password: 'correcthorse',
        }),
      }),
    )
  })

  it('createTopic sends the challenge when one is supplied', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic()))
    await createTopic('general', 'Hello', 'World', ['rust'], undefined, 'a blue moon')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/general/topics'),
      expect.objectContaining({
        body: JSON.stringify({
          title: 'Hello',
          body: 'World',
          tags: ['rust'],
          challenge: 'a blue moon',
        }),
      }),
    )
  })

  it('createTopic omits the challenge when none is supplied', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic()))
    await createTopic('general', 'Hello', 'World', ['rust'])
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/general/topics'),
      expect.objectContaining({
        body: JSON.stringify({ title: 'Hello', body: 'World', tags: ['rust'] }),
      }),
    )
  })

  it('createTopic omits the challenge when it is empty', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic()))
    await createTopic('general', 'Hello', 'World', ['rust'], undefined, '')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/general/topics'),
      expect.objectContaining({
        body: JSON.stringify({ title: 'Hello', body: 'World', tags: ['rust'] }),
      }),
    )
  })

  it('postComment sends the challenge when one is supplied', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawComment()))
    await postComment('t1', 'Nice topic!', null, 'a blue moon')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments'),
      expect.objectContaining({
        body: JSON.stringify({
          body: 'Nice topic!',
          parent_id: null,
          challenge: 'a blue moon',
        }),
      }),
    )
  })

  it('postComment omits the challenge when none is supplied', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawComment()))
    await postComment('t1', 'Nice topic!', null)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments'),
      expect.objectContaining({
        body: JSON.stringify({ body: 'Nice topic!', parent_id: null }),
      }),
    )
  })

  it('postComment omits the challenge when it is empty', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawComment()))
    await postComment('t1', 'Nice topic!', null, '')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1/comments'),
      expect.objectContaining({
        body: JSON.stringify({ body: 'Nice topic!', parent_id: null }),
      }),
    )
  })

  it('reports the status when register is refused with 428', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(428, { error: 'challenge required' }),
    )
    const result = await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(result).toEqual({ ok: false, error: 'challenge required', status: 428 })
  })

  it('reports the status when createTopic is refused with 428', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(428, { error: 'challenge required' }),
    )
    const result = await createTopic('general', 'Hello', 'World', ['rust'])
    expect(result).toEqual({ ok: false, error: 'challenge required', status: 428 })
  })

  it('reports the status when postComment is refused with 428', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(428, { error: 'challenge required' }),
    )
    const result = await postComment('t1', 'Nice topic!', null)
    expect(result).toEqual({ ok: false, error: 'challenge required', status: 428 })
  })
})

describe('content lifecycle', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps draft, sticky, off_front, resolved and minor from the topic response', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        rawTopic({
          draft: true,
          sticky: true,
          off_front: true,
          resolved: true,
          minor: true,
        }),
      ),
    )
    const result = await getTopic('1')
    expect(result.ok).toBe(true)
    expect(result.ok && result.value.draft).toBe(true)
    expect(result.ok && result.value.sticky).toBe(true)
    expect(result.ok && result.value.offFront).toBe(true)
    expect(result.ok && result.value.resolved).toBe(true)
    expect(result.ok && result.value.minor).toBe(true)
  })

  it('maps the lifecycle flags when they are false', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        rawTopic({
          draft: false,
          sticky: false,
          off_front: false,
          resolved: false,
          minor: false,
        }),
      ),
    )
    const result = await getTopic('1')
    expect(result.ok && result.value.draft).toBe(false)
    expect(result.ok && result.value.sticky).toBe(false)
    expect(result.ok && result.value.offFront).toBe(false)
    expect(result.ok && result.value.resolved).toBe(false)
    expect(result.ok && result.value.minor).toBe(false)
  })

  it('createTopic sends draft when it is true', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ draft: true })))
    await createTopic('general', 'Hello', 'World', ['rust'], undefined, undefined, true)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/general/topics'),
      expect.objectContaining({
        body: JSON.stringify({ title: 'Hello', body: 'World', tags: ['rust'], draft: true }),
      }),
    )
  })

  it('createTopic omits draft when it is false', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic()))
    await createTopic('general', 'Hello', 'World', ['rust'], undefined, undefined, false)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/general/topics'),
      expect.objectContaining({
        body: JSON.stringify({ title: 'Hello', body: 'World', tags: ['rust'] }),
      }),
    )
  })

  it('createTopic keeps the group and challenge alongside draft', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ draft: true })))
    await createTopic('general', 'Hello', 'World', ['rust'], 'announcements', 'a blue moon', true)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/general/topics'),
      expect.objectContaining({
        body: JSON.stringify({
          title: 'Hello',
          body: 'World',
          tags: ['rust'],
          group: 'announcements',
          draft: true,
          challenge: 'a blue moon',
        }),
      }),
    )
  })

  it('editTopic sends minor when it is true', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ minor: true })))
    await editTopic('t1', 'After', 'New', ['rust'], true)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1'),
      expect.objectContaining({
        method: 'PATCH',
        body: JSON.stringify({ title: 'After', body: 'New', tags: ['rust'], minor: true }),
      }),
    )
  })

  it('editTopic omits minor when it is false', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic()))
    await editTopic('t1', 'After', 'New', ['rust'], false)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t1'),
      expect.objectContaining({
        method: 'PATCH',
        body: JSON.stringify({ title: 'After', body: 'New', tags: ['rust'] }),
      }),
    )
  })

  it('publishTopic posts to the publish endpoint', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ id: '7', draft: false })))
    const result = await publishTopic('7')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/7/publish',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(result.ok && result.value.draft).toBe(false)
  })

  it('setSticky posts the sticky flag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ id: '7', sticky: true })))
    const result = await setSticky('7', true)
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/7/sticky',
      expect.objectContaining({ method: 'POST', body: JSON.stringify({ sticky: true }) }),
    )
    expect(result.ok && result.value.sticky).toBe(true)
  })

  it('setOffFront posts the off_front flag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ id: '7', off_front: true })))
    const result = await setOffFront('7', true)
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/7/off-front',
      expect.objectContaining({ method: 'POST', body: JSON.stringify({ off_front: true }) }),
    )
    expect(result.ok && result.value.offFront).toBe(true)
  })

  it('setResolved posts the resolved flag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTopic({ id: '7', resolved: false })))
    const result = await setResolved('7', false)
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/7/resolved',
      expect.objectContaining({ method: 'POST', body: JSON.stringify({ resolved: false }) }),
    )
    expect(result.ok && result.value.resolved).toBe(false)
  })

  it('reports the error when setSticky is refused with 403', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'not a moderator' }))
    const result = await setSticky('7', true)
    expect(result).toEqual({ ok: false, error: 'not a moderator', status: 403 })
  })

  it('reports the error when publishTopic is refused with 403', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'not the author' }))
    const result = await publishTopic('7')
    expect(result).toEqual({ ok: false, error: 'not the author', status: 403 })
  })
})

describe('getWatchState', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the watching envelope to a plain boolean', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { watching: true }))
    const result = await getWatchState('t1')
    expect(result).toEqual({ ok: true, value: true })
  })

  it('reads the watch state with GET', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { watching: false }))
    const result = await getWatchState('t1')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/t1/watch',
      expect.objectContaining({ method: 'GET' }),
    )
    expect(result).toEqual({ ok: true, value: false })
  })

  it('url-encodes the topic id', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { watching: false }))
    await getWatchState('t/1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/topics/t%2F1/watch'),
      expect.anything(),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getWatchState('t1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('watchTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts to the watch path of the topic', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { watching: true }))
    const result = await watchTopic('t1')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/t1/watch',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(result.ok).toBe(true)
  })

  it('reports the error when the topic is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(404, { error: 'topic not found' }))
    const result = await watchTopic('missing')
    expect(result).toEqual({ ok: false, error: 'topic not found', status: 404 })
  })
})

describe('unwatchTopic', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('deletes the watch path of the topic', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await unwatchTopic('t1')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/t1/watch',
      expect.objectContaining({ method: 'DELETE' }),
    )
    expect(result.ok).toBe(true)
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await unwatchTopic('t1')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getWatched', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('reads the watched listing with GET', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getWatched()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/watched',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('maps snake_case fields to the Topic type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawTopic()])))
    const result = await getWatched()
    expect(result.ok && result.value.items).toEqual([
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
    ])
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawTopic()], {
          number: 2,
          size: 25,
          total: 51,
          total_pages: 3,
          has_next: true,
          has_previous: true,
        }),
      ),
    )
    const result = await getWatched(2)
    expect(result.ok && result.value.page).toEqual({
      number: 2,
      size: 25,
      total: 51,
      totalPages: 3,
      hasNext: true,
      hasPrevious: true,
    })
  })

  it('carries the page and size in the query string', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getWatched(4, 100)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/watched?page=4&size=100'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getWatched()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('notification kind', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps a watch notification kind', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, pagedBody([rawNotification({ kind: 'watch' })])),
    )
    const result = await getNotifications()
    expect(result.ok && result.value.items).toEqual([
      {
        id: '1',
        topicId: 't1',
        topicTitle: 'Getting Started',
        commentId: 'c1',
        actorUsername: 'bob_02',
        createdAt: '2026-09-03T00:00:00Z',
        read: false,
        kind: 'watch',
      },
    ])
  })

  it('maps a reply notification kind', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, rawNotification({ kind: 'reply', read: true })),
    )
    const result = await markNotificationRead('1')
    expect(result.ok && result.value.kind).toBe('reply')
  })
})

function rawRemark(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    subject_username: 'bob_02',
    text: 'met at the meetup',
    created_at: '2026-09-03T00:00:00Z',
    ...overrides,
  }
}

describe('getRemark', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps a missing note to null', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { text: null }))
    const result = await getRemark('bob_02')
    expect(result).toEqual({ ok: true, value: null })
  })

  it('unwraps the note text to a plain string', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { text: 'met at the meetup' }))
    const result = await getRemark('bob_02')
    expect(result).toEqual({ ok: true, value: 'met at the meetup' })
  })

  it('reads the note with GET and url-encodes the username', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { text: null }))
    await getRemark('bob 02/x')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob%2002%2Fx/remark'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getRemark('bob_02')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('setRemark', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends the text with PUT to the note path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    await setRemark('bob_02', 'met at the meetup')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob_02/remark'),
      expect.objectContaining({
        method: 'PUT',
        body: JSON.stringify({ text: 'met at the meetup' }),
      }),
    )
  })

  it('url-encodes the username', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    await setRemark('bob 02/x', 'hello')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob%2002%2Fx/remark'),
      expect.anything(),
    )
  })

  it('returns the server error when the text is rejected', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'not about yourself' }))
    const result = await setRemark('alice_01', 'about me')
    expect(result).toEqual({ ok: false, error: 'not about yourself' })
  })
})

describe('clearRemark', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('uses DELETE on the note path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await clearRemark('bob_02')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob_02/remark'),
      expect.objectContaining({ method: 'DELETE' }),
    )
    expect(result.ok).toBe(true)
  })

  it('url-encodes the username', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    await clearRemark('bob 02/x')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/users/bob%2002%2Fx/remark'),
      expect.anything(),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await clearRemark('bob_02')
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

describe('getRemarks', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Remark type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawRemark()])))
    const result = await getRemarks()
    expect(result.ok && result.value.items).toEqual([
      {
        subjectUsername: 'bob_02',
        text: 'met at the meetup',
        createdAt: '2026-09-03T00:00:00Z',
      },
    ])
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawRemark()], {
          number: 2,
          size: 25,
          total: 51,
          total_pages: 3,
          has_next: true,
          has_previous: true,
        }),
      ),
    )
    const result = await getRemarks(2)
    expect(result.ok && result.value.page).toEqual({
      number: 2,
      size: 25,
      total: 51,
      totalPages: 3,
      hasNext: true,
      hasPrevious: true,
    })
  })

  it('reads the listing with GET and carries the page and size', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getRemarks(4, 100)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/remarks?page=4&size=100'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'missing session' }))
    const result = await getRemarks()
    expect(result).toEqual({ ok: false, error: 'missing session' })
  })
})

function rawInvitation(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    code: 'ABCD1234',
    expires_at: '2026-09-10T00:00:00Z',
    spent: false,
    spent_by: null,
    ...overrides,
  }
}

describe('getInvitationPolicy', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('unwraps the required envelope to a plain boolean', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { required: true }))
    const result = await getInvitationPolicy()
    expect(result).toEqual({ ok: true, value: true })
  })

  it('reports an open forum when no code is required', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, { required: false }))
    const result = await getInvitationPolicy()
    expect(result).toEqual({ ok: true, value: false })
  })

  it('reads the policy with GET', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { required: false }))
    await getInvitationPolicy()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/invitations/policy',
      expect.objectContaining({ method: 'GET', credentials: 'include' }),
    )
  })

  it('returns the server error when the policy cannot be read', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'service unavailable' }))
    const result = await getInvitationPolicy()
    expect(result).toEqual({ ok: false, error: 'service unavailable' })
  })
})

describe('issueInvitation', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Invitation type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawInvitation()))
    const result = await issueInvitation()
    expect(result).toEqual({
      ok: true,
      value: {
        code: 'ABCD1234',
        expiresAt: '2026-09-10T00:00:00Z',
        spent: false,
        spentBy: null,
      },
    })
  })

  it('posts to the invitations path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawInvitation()))
    await issueInvitation()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/invitations',
      expect.objectContaining({ method: 'POST', credentials: 'include' }),
    )
  })

  it('reports the status when too many codes are outstanding', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(422, { error: 'too many unused invitations' }),
    )
    const result = await issueInvitation()
    expect(result).toEqual({
      ok: false,
      error: 'too many unused invitations',
      status: 422,
    })
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(401, { error: 'missing session' }))
    const result = await issueInvitation()
    expect(result).toEqual({ ok: false, error: 'missing session', status: 401 })
  })
})

describe('getInvitations', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Invitation type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawInvitation()])))
    const result = await getInvitations()
    expect(result.ok && result.value.items).toEqual([
      {
        code: 'ABCD1234',
        expiresAt: '2026-09-10T00:00:00Z',
        spent: false,
        spentBy: null,
      },
    ])
  })

  it('carries the username that spent a code', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, pagedBody([rawInvitation({ spent: true, spent_by: 'bob_02' })])),
    )
    const result = await getInvitations()
    expect(result.ok).toBe(true)
    if (!result.ok) return
    const [first] = result.value.items
    expect(first?.spent).toBe(true)
    expect(first?.spentBy).toBe('bob_02')
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawInvitation()], {
          number: 2,
          size: 25,
          total: 51,
          total_pages: 3,
          has_next: true,
          has_previous: true,
        }),
      ),
    )
    const result = await getInvitations(2)
    expect(result.ok && result.value.page).toEqual({
      number: 2,
      size: 25,
      total: 51,
      totalPages: 3,
      hasNext: true,
      hasPrevious: true,
    })
  })

  it('reads the listing with GET and carries the page and size', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getInvitations(4, 100)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/invitations?page=4&size=100'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('omits the query string when no page is asked for', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getInvitations()
    expect(fetchMock).toHaveBeenCalledWith(
      expect.not.stringContaining('?'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an empty listing for an issuer with no codes', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([], { total_pages: 0 })))
    const result = await getInvitations()
    expect(result.ok && result.value.items).toEqual([])
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(401, { error: 'missing session' }))
    const result = await getInvitations()
    expect(result).toEqual({ ok: false, error: 'missing session', status: 401 })
  })
})

describe('register with an invitation', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('sends the invitation when one is supplied', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    await register('alice_01', 'alice@example.com', 'correcthorse', undefined, 'ABCD1234')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/register'),
      expect.objectContaining({
        body: JSON.stringify({
          username: 'alice_01',
          email: 'alice@example.com',
          password: 'correcthorse',
          invitation: 'ABCD1234',
        }),
      }),
    )
  })

  it('sends both the invitation and the challenge', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    await register('alice_01', 'alice@example.com', 'correcthorse', 'a blue moon', 'ABCD1234')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/register'),
      expect.objectContaining({
        body: JSON.stringify({
          username: 'alice_01',
          email: 'alice@example.com',
          password: 'correcthorse',
          invitation: 'ABCD1234',
          challenge: 'a blue moon',
        }),
      }),
    )
  })

  it('omits the invitation when it is empty', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, { id: '1', username: 'alice_01' }))
    await register('alice_01', 'alice@example.com', 'correcthorse', '', '')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/register'),
      expect.objectContaining({
        body: JSON.stringify({
          username: 'alice_01',
          email: 'alice@example.com',
          password: 'correcthorse',
        }),
      }),
    )
  })

  it('reports the status when a code is required but missing', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(403, { error: 'an invitation code is required' }),
    )
    const result = await register('alice_01', 'alice@example.com', 'correcthorse')
    expect(result).toEqual({
      ok: false,
      error: 'an invitation code is required',
      status: 403,
    })
  })

  it('reports an unknown code', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(403, { error: 'unknown invitation code' }),
    )
    const result = await register('alice_01', 'alice@example.com', 'correcthorse', '', 'nope')
    expect(result).toEqual({ ok: false, error: 'unknown invitation code', status: 403 })
  })

  it('reports a spent code', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(403, { error: 'invitation already used' }),
    )
    const result = await register('alice_01', 'alice@example.com', 'correcthorse', '', 'used')
    expect(result).toEqual({ ok: false, error: 'invitation already used', status: 403 })
  })

  it('reports an expired code', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'invitation expired' }))
    const result = await register('alice_01', 'alice@example.com', 'correcthorse', '', 'old')
    expect(result).toEqual({ ok: false, error: 'invitation expired', status: 403 })
  })
})

function rawSectionSettings(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    slug: 'general',
    title: 'General',
    topics_score: 'unrestricted',
    ...overrides,
  }
}

describe('TOPICS_SCORES', () => {
  it('offers exactly the standings the server accepts', () => {
    expect([...TOPICS_SCORES]).toEqual([
      'unrestricted',
      'registered',
      'moderator-or-author',
      'moderators-only',
      'no-comments',
      'floor-50',
      'floor-100',
      'floor-200',
      'floor-300',
      'floor-400',
      'floor-500',
    ])
  })
})

describe('createSection', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the SectionSettings type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawSectionSettings()))
    const result = await createSection('general', 'General')
    expect(result).toEqual({
      ok: true,
      value: { slug: 'general', title: 'General', topicsScore: 'unrestricted' },
    })
  })

  it('posts the slug and title to the sections path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawSectionSettings()))
    await createSection('general', 'General')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/sections',
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        body: JSON.stringify({ slug: 'general', title: 'General' }),
      }),
    )
  })

  it('reports the status when the slug is taken', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(409, { error: 'slug taken' }))
    const result = await createSection('general', 'General')
    expect(result).toEqual({ ok: false, error: 'slug taken', status: 409 })
  })

  it('reports the status when the slug is invalid', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(422, { error: 'invalid slug' }))
    const result = await createSection('Not A Slug', 'General')
    expect(result).toEqual({ ok: false, error: 'invalid slug', status: 422 })
  })

  it('reports the status when the caller is not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'not authorized' }))
    const result = await createSection('general', 'General')
    expect(result).toEqual({ ok: false, error: 'not authorized', status: 403 })
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(401, { error: 'missing session' }))
    const result = await createSection('general', 'General')
    expect(result).toEqual({ ok: false, error: 'missing session', status: 401 })
  })
})

describe('renameSection', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('patches the title on the settings path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawSectionSettings({ title: 'Chatter' })))
    const result = await renameSection('general', 'Chatter')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/sections/general/settings',
      expect.objectContaining({
        method: 'PATCH',
        credentials: 'include',
        body: JSON.stringify({ title: 'Chatter' }),
      }),
    )
    expect(result).toEqual({
      ok: true,
      value: { slug: 'general', title: 'Chatter', topicsScore: 'unrestricted' },
    })
  })

  it('url-encodes the slug', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawSectionSettings()))
    await renameSection('gen eral/x', 'General')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/gen%20eral%2Fx/settings'),
      expect.anything(),
    )
  })

  it('reports the status when the section is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(404, { error: 'section not found' }))
    const result = await renameSection('nope', 'Nope')
    expect(result).toEqual({ ok: false, error: 'section not found', status: 404 })
  })

  it('reports the status when the title is invalid', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(422, { error: 'invalid title' }))
    const result = await renameSection('general', '')
    expect(result).toEqual({ ok: false, error: 'invalid title', status: 422 })
  })

  it('reports the status when the caller is not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'not authorized' }))
    const result = await renameSection('general', 'Chatter')
    expect(result).toEqual({ ok: false, error: 'not authorized', status: 403 })
  })
})

describe('setSectionScore', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the standing name to the topics-score path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, rawSectionSettings({ topics_score: 'floor-100' })),
    )
    const result = await setSectionScore('general', 'floor-100')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/sections/general/topics-score',
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        body: JSON.stringify({ topics_score: 'floor-100' }),
      }),
    )
    expect(result).toEqual({
      ok: true,
      value: { slug: 'general', title: 'General', topicsScore: 'floor-100' },
    })
  })

  it('url-encodes the slug', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawSectionSettings()))
    await setSectionScore('gen eral/x', 'registered')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/gen%20eral%2Fx/topics-score'),
      expect.anything(),
    )
  })

  it('reports the status when the standing name is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(422, { error: 'invalid topics score' }))
    const result = await setSectionScore('general', '100')
    expect(result).toEqual({ ok: false, error: 'invalid topics score', status: 422 })
  })

  it('reports the status when the caller is not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'not authorized' }))
    const result = await setSectionScore('general', 'moderators-only')
    expect(result).toEqual({ ok: false, error: 'not authorized', status: 403 })
  })
})

describe('renameGroup', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('patches the title on the group path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, {
        id: 'g1',
        section_slug: 'general',
        name: 'Notices',
        slug: 'announcements',
      }),
    )
    const result = await renameGroup('general', 'announcements', 'Notices')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/sections/general/groups/announcements',
      expect.objectContaining({
        method: 'PATCH',
        credentials: 'include',
        body: JSON.stringify({ title: 'Notices' }),
      }),
    )
    expect(result.ok && result.value.name).toBe('Notices')
  })

  it('keeps the group slug when the name changes', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, {
        id: 'g1',
        section_slug: 'general',
        name: 'Notices',
        slug: 'announcements',
      }),
    )
    const result = await renameGroup('general', 'announcements', 'Notices')
    expect(result.ok && result.value.slug).toBe('announcements')
  })

  it('maps snake_case fields to the Group type', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, {
        id: 'g1',
        section_slug: 'general',
        name: 'Notices',
        slug: 'announcements',
      }),
    )
    const result = await renameGroup('general', 'announcements', 'Notices')
    expect(result).toEqual({
      ok: true,
      value: { id: 'g1', sectionSlug: 'general', name: 'Notices', slug: 'announcements' },
    })
  })

  it('url-encodes both slugs', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(
      jsonResponse(true, {
        id: 'g1',
        section_slug: 'general',
        name: 'Notices',
        slug: 'announcements',
      }),
    )
    await renameGroup('gen eral/x', 'ann ounce/y', 'Notices')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/sections/gen%20eral%2Fx/groups/ann%20ounce%2Fy'),
      expect.anything(),
    )
  })

  it('reports the status when the group is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(404, { error: 'group not found' }))
    const result = await renameGroup('general', 'nope', 'Notices')
    expect(result).toEqual({ ok: false, error: 'group not found', status: 404 })
  })

  it('reports the status when the caller is not a moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'not authorized' }))
    const result = await renameGroup('general', 'announcements', 'Notices')
    expect(result).toEqual({ ok: false, error: 'not authorized', status: 403 })
  })

  it('reports the status when the title is invalid', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(422, { error: 'invalid title' }))
    const result = await renameGroup('general', 'announcements', '')
    expect(result).toEqual({ ok: false, error: 'invalid title', status: 422 })
  })
})

describe('getArchiveMonths', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('reads the months with GET on the archive path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getArchiveMonths()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/archive',
      expect.objectContaining({ method: 'GET', credentials: 'include' }),
    )
  })

  it('returns the months newest first as the server sent them', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        { year: 2024, month: 7, topics: 2 },
        { year: 2024, month: 6, topics: 1 },
      ]),
    )
    const result = await getArchiveMonths()
    expect(result).toEqual({
      ok: true,
      value: [
        { year: 2024, month: 7, topics: 2 },
        { year: 2024, month: 6, topics: 1 },
      ],
    })
  })

  it('returns an empty listing when nothing is archived', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    const result = await getArchiveMonths()
    expect(result).toEqual({ ok: true, value: [] })
  })

  it('reports the status when the request fails', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(500, { error: 'archive unavailable' }))
    const result = await getArchiveMonths()
    expect(result).toEqual({ ok: false, error: 'archive unavailable', status: 500 })
  })
})

describe('getArchiveMonth', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Topic type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([rawTopic()])))
    const result = await getArchiveMonth(2024, 6)
    expect(result.ok && result.value.items).toEqual([
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
    ])
  })

  it('maps the page envelope to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(
        true,
        pagedBody([rawTopic()], {
          number: 2,
          size: 25,
          total: 51,
          total_pages: 3,
          has_next: true,
          has_previous: true,
        }),
      ),
    )
    const result = await getArchiveMonth(2024, 6, 2)
    expect(result.ok && result.value.page).toEqual({
      number: 2,
      size: 25,
      total: 51,
      totalPages: 3,
      hasNext: true,
      hasPrevious: true,
    })
  })

  it('omits the query string when no page is asked for', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getArchiveMonth(2024, 6)
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/archive/2024/6',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('carries the page and size in the query string', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, pagedBody([])))
    await getArchiveMonth(2024, 12, 3, 50)
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/archive/2024/12?page=3&size=50'),
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('returns an empty listing for a month that holds nothing', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, pagedBody([], { total_pages: 0 })))
    const result = await getArchiveMonth(2024, 6)
    expect(result.ok && result.value.items).toEqual([])
  })

  it('reports the status when the month is outside the calendar', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(422, { error: 'invalid month' }))
    const result = await getArchiveMonth(2024, 13)
    expect(result).toEqual({ ok: false, error: 'invalid month', status: 422 })
  })
})

function rawVersion(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: 'v1',
    title: 'Hello',
    body: 'World',
    editor: 'alice_01',
    written_at: '2026-09-03T00:00:00Z',
    ...overrides,
  }
}

describe('getTopicHistory', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields to the Version type', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, [rawVersion()]))
    const result = await getTopicHistory('t1')
    expect(result).toEqual({
      ok: true,
      value: [
        {
          id: 'v1',
          title: 'Hello',
          body: 'World',
          editor: 'alice_01',
          writtenAt: '2026-09-03T00:00:00Z',
        },
      ],
    })
  })

  it('keeps the versions in the order the server sent them', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        rawVersion({ id: 'v1', body: 'First', written_at: '2026-09-01T00:00:00Z' }),
        rawVersion({ id: 'v2', body: 'Second', written_at: '2026-09-02T00:00:00Z' }),
      ]),
    )
    const result = await getTopicHistory('t1')
    expect(result.ok && result.value.map((version) => version.id)).toEqual(['v1', 'v2'])
  })

  it('returns an empty listing for a topic that was never edited', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    const result = await getTopicHistory('t1')
    expect(result).toEqual({ ok: true, value: [] })
  })

  it('reads the history with GET and escapes the identifier', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getTopicHistory('t 1')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/t%201/history',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('sends credentials so the session cookie is carried', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getTopicHistory('t1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ credentials: 'include' }),
    )
  })

  it('reports the status when the topic is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(404, { error: 'topic not found' }))
    const result = await getTopicHistory('missing')
    expect(result).toEqual({ ok: false, error: 'topic not found', status: 404 })
  })
})

describe('getCommentHistory', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('maps snake_case fields and keeps the absent title', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [rawVersion({ title: null, body: 'Nice topic!' })]),
    )
    const result = await getCommentHistory('t1', 'c1')
    expect(result).toEqual({
      ok: true,
      value: [
        {
          id: 'v1',
          title: null,
          body: 'Nice topic!',
          editor: 'alice_01',
          writtenAt: '2026-09-03T00:00:00Z',
        },
      ],
    })
  })

  it('returns an empty listing for a comment that was never edited', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    const result = await getCommentHistory('t1', 'c1')
    expect(result).toEqual({ ok: true, value: [] })
  })

  it('reads the history with GET under the topic and comment', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getCommentHistory('t1', 'c1')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/t1/comments/c1/history',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('reports the status when the comment is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(404, { error: 'comment not found' }))
    const result = await getCommentHistory('t1', 'missing')
    expect(result).toEqual({ ok: false, error: 'comment not found', status: 404 })
  })
})

describe('getTopicDifference', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the lines with their kinds in the order the server sent them', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [
        { kind: 'kept', line: 'first line' },
        { kind: 'removed', line: 'old line' },
        { kind: 'added', line: 'new line' },
      ]),
    )
    const result = await getTopicDifference('t1', 'v1')
    expect(result).toEqual({
      ok: true,
      value: [
        { kind: 'kept', line: 'first line' },
        { kind: 'removed', line: 'old line' },
        { kind: 'added', line: 'new line' },
      ],
    })
  })

  it('returns an empty difference when nothing changed', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    const result = await getTopicDifference('t1', 'v1')
    expect(result).toEqual({ ok: true, value: [] })
  })

  it('reads the difference with GET and escapes both identifiers', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getTopicDifference('t 1', 'v 1')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/topics/t%201/history/v%201',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('sends credentials so the session cookie is carried', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getTopicDifference('t1', 'v1')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ credentials: 'include' }),
    )
  })

  it('reports the status when the version is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(404, { error: 'version not found' }))
    const result = await getTopicDifference('t1', 'missing')
    expect(result).toEqual({ ok: false, error: 'version not found', status: 404 })
  })
})

function rawTag(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    slug: 'rust',
    description: 'A language for reliable software.',
    means: null,
    following: false,
    ...overrides,
  }
}

describe('getTag', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the dictionary entry of the tag', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawTag()))
    const result = await getTag('rust')
    expect(result).toEqual({
      ok: true,
      value: {
        slug: 'rust',
        description: 'A language for reliable software.',
        means: null,
        following: false,
      },
    })
  })

  it('keeps an entry that carries neither description nor synonym', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, rawTag({ description: null, means: null })),
    )
    const result = await getTag('rust')
    expect(result.ok && result.value.description).toBe(null)
    expect(result.ok && result.value.means).toBe(null)
  })

  it('carries the tag a synonym means', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawTag({ slug: 'rustlang', means: 'rust' })))
    const result = await getTag('rustlang')
    expect(result.ok && result.value.means).toBe('rust')
  })

  it('reads the entry with GET', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTag()))
    await getTag('rust')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tags/rust',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('url-encodes the tag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTag()))
    await getTag('c/c++')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tags/c%2Fc%2B%2B',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('sends credentials so the session cookie is carried', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTag()))
    await getTag('rust')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ credentials: 'include' }),
    )
  })

  it('reports the status when the tag is unknown', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(404, { error: 'tag not found' }))
    const result = await getTag('missing')
    expect(result).toEqual({ ok: false, error: 'tag not found', status: 404 })
  })
})

describe('describeTag', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('patches the tag with the description', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTag({ description: 'Systems work.' })))
    await describeTag('rust', 'Systems work.')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tags/rust',
      expect.objectContaining({
        method: 'PATCH',
        body: JSON.stringify({ description: 'Systems work.' }),
      }),
    )
  })

  it('returns the entry the server wrote', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawTag({ description: 'Systems work.' })))
    const result = await describeTag('rust', 'Systems work.')
    expect(result.ok && result.value.description).toBe('Systems work.')
  })

  it('reports the status when the viewer is no moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'moderator only' }))
    const result = await describeTag('rust', 'Systems work.')
    expect(result).toEqual({ ok: false, error: 'moderator only', status: 403 })
  })

  it('reports the status when the description is empty', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(422, { error: 'description is empty' }))
    const result = await describeTag('rust', '')
    expect(result).toEqual({ ok: false, error: 'description is empty', status: 422 })
  })
})

describe('makeSynonym', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts the tag it means under the means path', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTag({ slug: 'rustlang', means: 'rust' })))
    await makeSynonym('rustlang', 'rust')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tags/rustlang/means',
      expect.objectContaining({ method: 'POST', body: JSON.stringify({ means: 'rust' }) }),
    )
  })

  it('returns the entry carrying the tag it now means', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, rawTag({ slug: 'rustlang', means: 'rust' })))
    const result = await makeSynonym('rustlang', 'rust')
    expect(result.ok && result.value.means).toBe('rust')
  })

  it('url-encodes the tag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, rawTag({ means: 'rust' })))
    await makeSynonym('c/c++', 'rust')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tags/c%2Fc%2B%2B/means',
      expect.anything(),
    )
  })

  it('reports the status when a tag would mean itself', async () => {
    vi.mocked(fetch).mockResolvedValue(
      statusResponse(422, { error: 'a tag cannot mean itself' }),
    )
    const result = await makeSynonym('rust', 'rust')
    expect(result).toEqual({ ok: false, error: 'a tag cannot mean itself', status: 422 })
  })

  it('reports the status when the viewer is no moderator', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(403, { error: 'moderator only' }))
    const result = await makeSynonym('rustlang', 'rust')
    expect(result).toEqual({ ok: false, error: 'moderator only', status: 403 })
  })
})

describe('followTag', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('posts to the follow path of the tag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await followTag('rust')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tags/rust/follow',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(result.ok).toBe(true)
  })

  it('sends credentials so the session cookie is carried', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    await followTag('rust')
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ credentials: 'include' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(401, { error: 'missing session' }))
    const result = await followTag('rust')
    expect(result).toEqual({ ok: false, error: 'missing session', status: 401 })
  })
})

describe('unfollowTag', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('deletes the follow path of the tag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    const result = await unfollowTag('rust')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tags/rust/follow',
      expect.objectContaining({ method: 'DELETE' }),
    )
    expect(result.ok).toBe(true)
  })

  it('url-encodes the tag', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(emptyResponse(true))
    await unfollowTag('c/c++')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tags/c%2Fc%2B%2B/follow',
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(401, { error: 'missing session' }))
    const result = await unfollowTag('rust')
    expect(result).toEqual({ ok: false, error: 'missing session', status: 401 })
  })
})

describe('getFollowedTags', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('returns the slugs in the order the server sent them', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, ['rust', 'vim']))
    const result = await getFollowedTags()
    expect(result).toEqual({ ok: true, value: ['rust', 'vim'] })
  })

  it('returns an empty listing when the viewer follows nothing', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    const result = await getFollowedTags()
    expect(result).toEqual({ ok: true, value: [] })
  })

  it('reads the listing with GET', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getFollowedTags()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/followed-tags',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('sends credentials so the session cookie is carried', async () => {
    const fetchMock = vi.mocked(fetch)
    fetchMock.mockResolvedValue(jsonResponse(true, []))
    await getFollowedTags()
    expect(fetchMock).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ credentials: 'include' }),
    )
  })

  it('returns an error when not authenticated', async () => {
    vi.mocked(fetch).mockResolvedValue(statusResponse(401, { error: 'missing session' }))
    const result = await getFollowedTags()
    expect(result).toEqual({ ok: false, error: 'missing session', status: 401 })
  })
})

describe('topic images', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('reads a subject images and names them in the way the app uses', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, [{ id: 'i1', content_type: 'image/png', uploaded_by: 'alice_01' }]),
    )
    const result = await getTopicImages('t1')
    expect(result).toEqual({
      ok: true,
      value: [{ id: 'i1', contentType: 'image/png', uploadedBy: 'alice_01' }],
    })
  })

  it('sends the bytes as data when attaching', async () => {
    vi.mocked(fetch).mockResolvedValue(
      jsonResponse(true, { id: 'i1', content_type: 'image/png', uploaded_by: 'alice_01' }),
    )
    await attachImage('t1', 'AAAA')
    const [, init] = vi.mocked(fetch).mock.calls[0] as [string, RequestInit]
    expect(init.method).toBe('POST')
    expect(init.body).toBe(JSON.stringify({ data: 'AAAA' }))
  })

  it('hands back what the server said when an upload is refused', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(false, { error: 'unsupported image' }))
    const result = await attachImage('t1', 'AAAA')
    expect(result).toEqual({ ok: false, error: 'unsupported image' })
  })

  it('removes one by its own address', async () => {
    vi.mocked(fetch).mockResolvedValue(emptyResponse(true))
    const result = await removeImage('t1', 'i1')
    expect(result.ok).toBe(true)
    const [path, init] = vi.mocked(fetch).mock.calls[0] as [string, RequestInit]
    expect(path).toContain('/api/topics/t1/images/i1')
    expect(init.method).toBe('DELETE')
  })

  it('builds an address an image tag can load', () => {
    expect(topicImageUrl('t1', 'i1')).toContain('/api/topics/t1/images/i1')
  })
})

describe('search narrowing', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  it('reads back every narrowing it offers', () => {
    expect(toSearchScope('everything')).toBe('everything')
    expect(toSearchScope('topics')).toBe('topics')
    expect(toSearchScope('comments')).toBe('comments')
    expect(toSearchOrder('relevance')).toBe('relevance')
    expect(toSearchOrder('newest')).toBe('newest')
    expect(toSearchOrder('oldest')).toBe('oldest')
  })

  it('falls back to the whole board in best-match order', () => {
    expect(toSearchScope('drafts')).toBe('everything')
    expect(toSearchScope(undefined)).toBe('everything')
    expect(toSearchScope(['topics'])).toBe('everything')
    expect(toSearchOrder('loudest')).toBe('relevance')
    expect(toSearchOrder(undefined)).toBe('relevance')
  })

  it('asks the server for what was chosen', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    await search('adapters', 'comments', 'oldest')
    const [path] = vi.mocked(fetch).mock.calls[0] as [string, RequestInit]
    expect(path).toContain('q=adapters')
    expect(path).toContain('scope=comments')
    expect(path).toContain('order=oldest')
  })

  it('asks for everything by best match when nothing was chosen', async () => {
    vi.mocked(fetch).mockResolvedValue(jsonResponse(true, []))
    await search('adapters')
    const [path] = vi.mocked(fetch).mock.calls[0] as [string, RequestInit]
    expect(path).toContain('scope=everything')
    expect(path).toContain('order=relevance')
  })
})
