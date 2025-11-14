export type User = { id: string; username: string; role: string }
export type Profile = { id: string; username: string; bio: string | null }
export type Section = { slug: string; title: string }
export type Topic = {
  id: string
  sectionSlug: string
  title: string
  body: string
  tags: string[]
  authorUsername: string
  createdAt: string
  deleted: boolean
  deletedReason: string | null
  edited: boolean
}
export type Comment = {
  id: string
  topicId: string
  parentId: string | null
  body: string
  authorUsername: string
  createdAt: string
  deleted: boolean
  deletedReason: string | null
  edited: boolean
}
export type Notification = {
  id: string
  topicId: string
  topicTitle: string
  commentId: string
  actorUsername: string
  createdAt: string
  read: boolean
}
export type ApiResult<T> = { ok: true; value: T } | { ok: false; error: string }

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? ''

async function request<T>(path: string, init: RequestInit): Promise<ApiResult<T>> {
  const response = await fetch(`${BASE_URL}${path}`, {
    credentials: 'include',
    ...init,
  })
  const text = await response.text()
  const data = text.length > 0 ? JSON.parse(text) : undefined
  if (!response.ok) {
    return { ok: false, error: data?.error ?? 'request failed' }
  }
  return { ok: true, value: data as T }
}

function post<T>(path: string, body: unknown): Promise<ApiResult<T>> {
  return request<T>(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
}

export function register(
  username: string,
  email: string,
  password: string,
): Promise<ApiResult<User>> {
  return post<User>('/api/register', { username, email, password })
}

export function signIn(username: string, password: string): Promise<ApiResult<User>> {
  return post<User>('/api/sign-in', { username, password })
}

export function me(): Promise<ApiResult<User>> {
  return request<User>('/api/me', { method: 'GET' })
}

export function signOut(): Promise<ApiResult<void>> {
  return request<void>('/api/sign-out', { method: 'POST' })
}

export function getProfile(username: string): Promise<ApiResult<Profile>> {
  return request<Profile>(`/api/users/${encodeURIComponent(username)}`, { method: 'GET' })
}

export function updateBio(bio: string): Promise<ApiResult<Profile>> {
  return request<Profile>('/api/me/bio', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ bio: bio.length > 0 ? bio : null }),
  })
}

type RawTopic = {
  id: string
  section_slug: string
  title: string
  body: string
  tags: string[]
  author_username: string
  created_at: string
  deleted: boolean
  deleted_reason: string | null
  edited: boolean
}

function toTopic(raw: RawTopic): Topic {
  return {
    id: raw.id,
    sectionSlug: raw.section_slug,
    title: raw.title,
    body: raw.body,
    tags: raw.tags,
    authorUsername: raw.author_username,
    createdAt: raw.created_at,
    deleted: raw.deleted,
    deletedReason: raw.deleted_reason,
    edited: raw.edited,
  }
}

export function getSections(): Promise<ApiResult<Section[]>> {
  return request<Section[]>('/api/sections', { method: 'GET' })
}

export async function getTopics(slug: string): Promise<ApiResult<Topic[]>> {
  const result = await request<RawTopic[]>(`/api/sections/${encodeURIComponent(slug)}/topics`, {
    method: 'GET',
  })
  return result.ok ? { ok: true, value: result.value.map(toTopic) } : result
}

export async function createTopic(
  slug: string,
  title: string,
  body: string,
  tags: string[],
): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/sections/${encodeURIComponent(slug)}/topics`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, body, tags }),
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function getTopic(id: string): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}`, {
    method: 'GET',
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function getTopicsByTag(tag: string): Promise<ApiResult<Topic[]>> {
  const result = await request<RawTopic[]>(`/api/tags/${encodeURIComponent(tag)}/topics`, {
    method: 'GET',
  })
  return result.ok ? { ok: true, value: result.value.map(toTopic) } : result
}

type RawComment = {
  id: string
  topic_id: string
  parent_id: string | null
  body: string
  author_username: string
  created_at: string
  deleted: boolean
  deleted_reason: string | null
  edited: boolean
}

function toComment(raw: RawComment): Comment {
  return {
    id: raw.id,
    topicId: raw.topic_id,
    parentId: raw.parent_id,
    body: raw.body,
    authorUsername: raw.author_username,
    createdAt: raw.created_at,
    deleted: raw.deleted,
    deletedReason: raw.deleted_reason,
    edited: raw.edited,
  }
}

export async function getComments(topicId: string): Promise<ApiResult<Comment[]>> {
  const result = await request<RawComment[]>(
    `/api/topics/${encodeURIComponent(topicId)}/comments`,
    { method: 'GET' },
  )
  return result.ok ? { ok: true, value: result.value.map(toComment) } : result
}

export async function postComment(
  topicId: string,
  body: string,
  parentId: string | null,
): Promise<ApiResult<Comment>> {
  const result = await request<RawComment>(
    `/api/topics/${encodeURIComponent(topicId)}/comments`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ body, parent_id: parentId }),
    },
  )
  return result.ok ? { ok: true, value: toComment(result.value) } : result
}

export async function deleteTopic(id: string, reason: string): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}/delete`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ reason }),
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function deleteComment(
  topicId: string,
  id: string,
  reason: string,
): Promise<ApiResult<Comment>> {
  const result = await request<RawComment>(
    `/api/topics/${encodeURIComponent(topicId)}/comments/${encodeURIComponent(id)}/delete`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason }),
    },
  )
  return result.ok ? { ok: true, value: toComment(result.value) } : result
}

export async function editTopic(
  id: string,
  title: string,
  body: string,
  tags: string[],
): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, body, tags }),
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function editComment(
  topicId: string,
  id: string,
  body: string,
): Promise<ApiResult<Comment>> {
  const result = await request<RawComment>(
    `/api/topics/${encodeURIComponent(topicId)}/comments/${encodeURIComponent(id)}`,
    {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ body }),
    },
  )
  return result.ok ? { ok: true, value: toComment(result.value) } : result
}

export type SearchHit =
  | { kind: 'topic'; topic: Topic }
  | { kind: 'comment'; comment: Comment }

type RawSearchHit =
  | ({ kind: 'topic' } & RawTopic)
  | ({ kind: 'comment' } & RawComment)

export async function search(query: string): Promise<ApiResult<SearchHit[]>> {
  const result = await request<RawSearchHit[]>(
    `/api/search?q=${encodeURIComponent(query)}`,
    { method: 'GET' },
  )
  if (!result.ok) return result
  return {
    ok: true,
    value: result.value.map((raw) =>
      raw.kind === 'topic'
        ? { kind: 'topic', topic: toTopic(raw) }
        : { kind: 'comment', comment: toComment(raw) },
    ),
  }
}

type RawNotification = {
  id: string
  topic_id: string
  topic_title: string
  comment_id: string
  actor_username: string
  created_at: string
  read: boolean
}

function toNotification(raw: RawNotification): Notification {
  return {
    id: raw.id,
    topicId: raw.topic_id,
    topicTitle: raw.topic_title,
    commentId: raw.comment_id,
    actorUsername: raw.actor_username,
    createdAt: raw.created_at,
    read: raw.read,
  }
}

export async function getNotifications(): Promise<ApiResult<Notification[]>> {
  const result = await request<RawNotification[]>('/api/notifications', { method: 'GET' })
  return result.ok ? { ok: true, value: result.value.map(toNotification) } : result
}

export async function getUnreadCount(): Promise<ApiResult<number>> {
  const result = await request<{ unread: number }>('/api/notifications/unread-count', {
    method: 'GET',
  })
  return result.ok ? { ok: true, value: result.value.unread } : result
}

export async function markNotificationRead(id: string): Promise<ApiResult<Notification>> {
  const result = await request<RawNotification>(
    `/api/notifications/${encodeURIComponent(id)}/read`,
    {
      method: 'POST',
    },
  )
  return result.ok ? { ok: true, value: toNotification(result.value) } : result
}
