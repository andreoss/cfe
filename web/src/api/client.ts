export type Role = 'user' | 'corrector' | 'moderator'
export type User = { id: string; username: string; role: Role }
export type Profile = {
  id: string
  username: string
  bio: string | null
  score: number
  role: Role
}
export type Section = { slug: string; title: string }
export type Group = {
  id: string
  sectionSlug: string
  name: string
  slug: string
}
export type Topic = {
  id: string
  sectionSlug: string
  groupSlug: string | null
  title: string
  body: string
  tags: string[]
  authorUsername: string
  createdAt: string
  deleted: boolean
  deletedReason: string | null
  edited: boolean
  postscore: number
  pending: boolean
  openReports: number
  draft: boolean
  sticky: boolean
  offFront: boolean
  resolved: boolean
  minor: boolean
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
  ignored: boolean
}
export type Notification = {
  id: string
  topicId: string
  topicTitle: string
  commentId: string
  actorUsername: string
  createdAt: string
  read: boolean
  kind: 'reply' | 'watch'
}
export type PollOption = { id: string; text: string; votes: number }
export type Poll = {
  id: string
  topicId: string
  question: string
  options: PollOption[]
  mine: string | null
  totalVotes: number
}
export type Warning = {
  id: string
  reason: string
  createdAt: string
  acknowledged: boolean
}
export type Ban = { reason: string; until: string | null }
export type AddressBlock = {
  addr: string
  reason: string
  blockedAt: string
  until: string | null
}
export type AddressPost = {
  username: string
  addr: string
  client: string | null
  at: string
}
export type ReportKind = 'rule' | 'spelling' | 'tag' | 'group'
export type Report = {
  id: string
  topicId: string
  commentId: string | null
  reporterUsername: string
  kind: ReportKind
  reason: string
  createdAt: string
}
export type ApiResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: string; status?: number }
export type PageInfo = {
  number: number
  size: number
  total: number
  totalPages: number
  hasNext: boolean
  hasPrevious: boolean
}
export type Paged<T> = { items: T[]; page: PageInfo }

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? ''

async function request<T>(path: string, init: RequestInit): Promise<ApiResult<T>> {
  const response = await fetch(`${BASE_URL}${path}`, {
    credentials: 'include',
    ...init,
  })
  const text = await response.text()
  const data = text.length > 0 ? JSON.parse(text) : undefined
  if (!response.ok) {
    return { ok: false, error: data?.error ?? 'request failed', status: response.status }
  }
  return { ok: true, value: data as T }
}

function withChallenge(
  body: Record<string, unknown>,
  challenge?: string,
): Record<string, unknown> {
  if (typeof challenge !== 'string' || challenge.length === 0) return body
  return { ...body, challenge }
}

function post<T>(path: string, body: unknown): Promise<ApiResult<T>> {
  return request<T>(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
}

type RawPageInfo = {
  number: number
  size: number
  total: number
  total_pages: number
  has_next: boolean
  has_previous: boolean
}

type RawPaged<T> = { items: T[]; page: RawPageInfo }

function toPageInfo(raw: RawPageInfo): PageInfo {
  return {
    number: raw.number,
    size: raw.size,
    total: raw.total,
    totalPages: raw.total_pages,
    hasNext: raw.has_next,
    hasPrevious: raw.has_previous,
  }
}

function pageQuery(page?: number, size?: number): string {
  const params = new URLSearchParams()
  if (page !== undefined) params.set('page', String(page))
  if (size !== undefined) params.set('size', String(size))
  const query = params.toString()
  return query.length > 0 ? `?${query}` : ''
}

async function requestPage<R, T>(
  path: string,
  map: (raw: R) => T,
): Promise<ApiResult<Paged<T>>> {
  const result = await request<RawPaged<R>>(path, { method: 'GET' })
  if (!result.ok) return result
  return {
    ok: true,
    value: { items: result.value.items.map(map), page: toPageInfo(result.value.page) },
  }
}

export function register(
  username: string,
  email: string,
  password: string,
  challenge?: string,
): Promise<ApiResult<User>> {
  return post<User>('/api/register', withChallenge({ username, email, password }, challenge))
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

export function endAllSessions(): Promise<ApiResult<void>> {
  return request<void>('/api/me/sessions/end-all', { method: 'POST' })
}

export async function changePassword(
  currentPassword: string,
  newPassword: string,
): Promise<ApiResult<User>> {
  return request<User>('/api/me/password', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }),
  })
}

export async function deregister(): Promise<ApiResult<User>> {
  return request<User>('/api/me/deregister', { method: 'POST' })
}

export function requestPasswordReset(email: string): Promise<ApiResult<void>> {
  return post<void>('/api/password-reset', { email })
}

export function confirmPasswordReset(
  code: string,
  newPassword: string,
): Promise<ApiResult<void>> {
  return post<void>('/api/password-reset/confirm', { code, new_password: newPassword })
}

export function requestEmailChange(email: string): Promise<ApiResult<void>> {
  return post<void>('/api/me/email', { email })
}

export function confirmEmailChange(code: string): Promise<ApiResult<User>> {
  return post<User>('/api/me/email/confirm', { code })
}

export function activateAccount(code: string): Promise<ApiResult<User>> {
  return post<User>('/api/activate', { code })
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
  group_slug: string | null
  title: string
  body: string
  tags: string[]
  author_username: string
  created_at: string
  deleted: boolean
  deleted_reason: string | null
  edited: boolean
  postscore: number
  pending: boolean
  open_reports: number
  draft: boolean
  sticky: boolean
  off_front: boolean
  resolved: boolean
  minor: boolean
}

function toTopic(raw: RawTopic): Topic {
  return {
    id: raw.id,
    sectionSlug: raw.section_slug,
    groupSlug: raw.group_slug,
    title: raw.title,
    body: raw.body,
    tags: raw.tags,
    authorUsername: raw.author_username,
    createdAt: raw.created_at,
    deleted: raw.deleted,
    deletedReason: raw.deleted_reason,
    edited: raw.edited,
    postscore: raw.postscore,
    pending: raw.pending,
    openReports: raw.open_reports,
    draft: raw.draft,
    sticky: raw.sticky,
    offFront: raw.off_front,
    resolved: raw.resolved,
    minor: raw.minor,
  }
}

export function getSections(): Promise<ApiResult<Section[]>> {
  return request<Section[]>('/api/sections', { method: 'GET' })
}

export function getTopics(
  slug: string,
  page?: number,
  size?: number,
): Promise<ApiResult<Paged<Topic>>> {
  return requestPage<RawTopic, Topic>(
    `/api/sections/${encodeURIComponent(slug)}/topics${pageQuery(page, size)}`,
    toTopic,
  )
}

export async function createTopic(
  slug: string,
  title: string,
  body: string,
  tags: string[],
  group?: string,
  challenge?: string,
  draft?: boolean,
): Promise<ApiResult<Topic>> {
  const payload: Record<string, unknown> = { title, body, tags, group }
  if (draft === true) payload.draft = true
  const result = await request<RawTopic>(`/api/sections/${encodeURIComponent(slug)}/topics`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(withChallenge(payload, challenge)),
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function getTopic(id: string): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}`, {
    method: 'GET',
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export function getGroups(slug: string): Promise<ApiResult<Group[]>> {
  return request<Group[]>(`/api/sections/${encodeURIComponent(slug)}/groups`, { method: 'GET' })
}

export async function createGroup(
  slug: string,
  name: string,
  groupSlug: string,
): Promise<ApiResult<Group>> {
  return request<Group>(`/api/sections/${encodeURIComponent(slug)}/groups`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, slug: groupSlug }),
  })
}

export async function commitTopic(id: string): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}/commit`, {
    method: 'POST',
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function uncommitTopic(id: string): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}/uncommit`, {
    method: 'POST',
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function moveTopic(id: string, group: string): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}/move`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ group }),
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export function getTopicsByTag(
  tag: string,
  page?: number,
  size?: number,
): Promise<ApiResult<Paged<Topic>>> {
  return requestPage<RawTopic, Topic>(
    `/api/tags/${encodeURIComponent(tag)}/topics${pageQuery(page, size)}`,
    toTopic,
  )
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
  ignored: boolean
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
    ignored: raw.ignored,
  }
}

export function getComments(
  topicId: string,
  page?: number,
  size?: number,
): Promise<ApiResult<Paged<Comment>>> {
  return requestPage<RawComment, Comment>(
    `/api/topics/${encodeURIComponent(topicId)}/comments${pageQuery(page, size)}`,
    toComment,
  )
}

export async function postComment(
  topicId: string,
  body: string,
  parentId: string | null,
  challenge?: string,
): Promise<ApiResult<Comment>> {
  const result = await request<RawComment>(
    `/api/topics/${encodeURIComponent(topicId)}/comments`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(withChallenge({ body, parent_id: parentId }, challenge)),
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

export async function setPostscore(id: string, postscore: number): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}/postscore`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ postscore }),
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
  minor?: boolean,
): Promise<ApiResult<Topic>> {
  const payload: Record<string, unknown> = { title, body, tags }
  if (minor === true) payload.minor = true
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export async function publishTopic(id: string): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(`/api/topics/${encodeURIComponent(id)}/publish`, {
    method: 'POST',
  })
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

async function topicFlag(
  id: string,
  segment: string,
  body: Record<string, boolean>,
): Promise<ApiResult<Topic>> {
  const result = await request<RawTopic>(
    `/api/topics/${encodeURIComponent(id)}/${segment}`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    },
  )
  return result.ok ? { ok: true, value: toTopic(result.value) } : result
}

export function setSticky(id: string, sticky: boolean): Promise<ApiResult<Topic>> {
  return topicFlag(id, 'sticky', { sticky })
}

export function setOffFront(id: string, offFront: boolean): Promise<ApiResult<Topic>> {
  return topicFlag(id, 'off-front', { off_front: offFront })
}

export function setResolved(id: string, resolved: boolean): Promise<ApiResult<Topic>> {
  return topicFlag(id, 'resolved', { resolved })
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

function toSearchHit(raw: RawSearchHit): SearchHit {
  return raw.kind === 'topic'
    ? { kind: 'topic', topic: toTopic(raw) }
    : { kind: 'comment', comment: toComment(raw) }
}

export async function search(query: string): Promise<ApiResult<SearchHit[]>> {
  const result = await request<RawSearchHit[]>(
    `/api/search?q=${encodeURIComponent(query)}`,
    { method: 'GET' },
  )
  if (!result.ok) return result
  return { ok: true, value: result.value.map(toSearchHit) }
}

export async function getActivity(): Promise<ApiResult<SearchHit[]>> {
  const result = await request<RawSearchHit[]>('/api/activity', { method: 'GET' })
  return result.ok ? { ok: true, value: result.value.map(toSearchHit) } : result
}

type RawNotification = {
  id: string
  topic_id: string
  topic_title: string
  comment_id: string
  actor_username: string
  created_at: string
  read: boolean
  kind: 'reply' | 'watch'
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
    kind: raw.kind,
  }
}

export function getNotifications(
  page?: number,
  size?: number,
): Promise<ApiResult<Paged<Notification>>> {
  return requestPage<RawNotification, Notification>(
    `/api/notifications${pageQuery(page, size)}`,
    toNotification,
  )
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

export const REACTION_KINDS = ['like', 'agree', 'disagree', 'thanks'] as const

export type ReactionCount = { kind: string; count: number }
export type ReactionSummary = { counts: ReactionCount[]; mine: string | null }

type RawReactions = { counts?: ReactionCount[]; mine?: string | null }

function toReactionSummary(raw: RawReactions): ReactionSummary {
  return { counts: raw.counts ?? [], mine: raw.mine ?? null }
}

function topicReactionsPath(topicId: string): string {
  return `/api/topics/${encodeURIComponent(topicId)}/reactions`
}

function commentReactionsPath(topicId: string, commentId: string): string {
  return `/api/topics/${encodeURIComponent(topicId)}/comments/${encodeURIComponent(
    commentId,
  )}/reactions`
}

async function reactionRequest(
  path: string,
  init: RequestInit,
): Promise<ApiResult<ReactionSummary>> {
  const result = await request<RawReactions>(path, init)
  return result.ok ? { ok: true, value: toReactionSummary(result.value) } : result
}

export function getTopicReactions(topicId: string): Promise<ApiResult<ReactionSummary>> {
  return reactionRequest(topicReactionsPath(topicId), { method: 'GET' })
}

export function reactToTopic(
  topicId: string,
  kind: string,
): Promise<ApiResult<ReactionSummary>> {
  return reactionRequest(topicReactionsPath(topicId), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ kind }),
  })
}

export function clearTopicReaction(topicId: string): Promise<ApiResult<ReactionSummary>> {
  return reactionRequest(topicReactionsPath(topicId), { method: 'DELETE' })
}

export function getCommentReactions(
  topicId: string,
  commentId: string,
): Promise<ApiResult<ReactionSummary>> {
  return reactionRequest(commentReactionsPath(topicId, commentId), { method: 'GET' })
}

export function reactToComment(
  topicId: string,
  commentId: string,
  kind: string,
): Promise<ApiResult<ReactionSummary>> {
  return reactionRequest(commentReactionsPath(topicId, commentId), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ kind }),
  })
}

export function clearCommentReaction(
  topicId: string,
  commentId: string,
): Promise<ApiResult<ReactionSummary>> {
  return reactionRequest(commentReactionsPath(topicId, commentId), { method: 'DELETE' })
}

export function getBookmarks(page?: number, size?: number): Promise<ApiResult<Paged<Topic>>> {
  return requestPage<RawTopic, Topic>(`/api/bookmarks${pageQuery(page, size)}`, toTopic)
}

export async function getBookmarkState(topicId: string): Promise<ApiResult<boolean>> {
  const result = await request<{ bookmarked: boolean }>(
    `/api/topics/${encodeURIComponent(topicId)}/bookmark`,
    { method: 'GET' },
  )
  return result.ok ? { ok: true, value: result.value.bookmarked } : result
}

export async function addBookmark(topicId: string): Promise<ApiResult<boolean>> {
  const result = await request<{ bookmarked: boolean }>(
    `/api/topics/${encodeURIComponent(topicId)}/bookmark`,
    { method: 'POST' },
  )
  return result.ok ? { ok: true, value: result.value.bookmarked } : result
}

export async function removeBookmark(topicId: string): Promise<ApiResult<boolean>> {
  const result = await request<{ bookmarked: boolean }>(
    `/api/topics/${encodeURIComponent(topicId)}/bookmark`,
    { method: 'DELETE' },
  )
  return result.ok ? { ok: true, value: result.value.bookmarked } : result
}

function watchPath(topicId: string): string {
  return `/api/topics/${encodeURIComponent(topicId)}/watch`
}

export function getWatched(page?: number, size?: number): Promise<ApiResult<Paged<Topic>>> {
  return requestPage<RawTopic, Topic>(`/api/watched${pageQuery(page, size)}`, toTopic)
}

export async function getWatchState(topicId: string): Promise<ApiResult<boolean>> {
  const result = await request<{ watching: boolean }>(watchPath(topicId), { method: 'GET' })
  return result.ok ? { ok: true, value: result.value.watching } : result
}

export function watchTopic(topicId: string): Promise<ApiResult<void>> {
  return request<void>(watchPath(topicId), { method: 'POST' })
}

export function unwatchTopic(topicId: string): Promise<ApiResult<void>> {
  return request<void>(watchPath(topicId), { method: 'DELETE' })
}

type RawPoll = {
  id: string
  topic_id: string
  question: string
  options: PollOption[]
  mine: string | null
  total_votes: number
}

function toPoll(raw: RawPoll): Poll {
  return {
    id: raw.id,
    topicId: raw.topic_id,
    question: raw.question,
    options: raw.options,
    mine: raw.mine,
    totalVotes: raw.total_votes,
  }
}

function pollPath(topicId: string): string {
  return `/api/topics/${encodeURIComponent(topicId)}/poll`
}

export async function getPoll(topicId: string): Promise<ApiResult<Poll>> {
  const result = await request<RawPoll>(pollPath(topicId), { method: 'GET' })
  return result.ok ? { ok: true, value: toPoll(result.value) } : result
}

export async function createPoll(
  topicId: string,
  question: string,
  options: string[],
): Promise<ApiResult<Poll>> {
  const result = await request<RawPoll>(pollPath(topicId), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ question, options }),
  })
  return result.ok ? { ok: true, value: toPoll(result.value) } : result
}

export async function votePoll(topicId: string, optionId: string): Promise<ApiResult<Poll>> {
  const result = await request<RawPoll>(`${pollPath(topicId)}/vote`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ option_id: optionId }),
  })
  return result.ok ? { ok: true, value: toPoll(result.value) } : result
}

export function sectionFeedUrl(slug: string): string {
  return `${BASE_URL}/api/sections/${encodeURIComponent(slug)}/feed`
}

export function tagFeedUrl(tag: string): string {
  return `${BASE_URL}/api/tags/${encodeURIComponent(tag)}/feed`
}

export function avatarUrl(username: string): string {
  return `${BASE_URL}/api/users/${encodeURIComponent(username)}/avatar`
}

export async function uploadAvatar(base64: string): Promise<ApiResult<boolean>> {
  const result = await request<{ has_avatar: boolean }>('/api/me/avatar', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ data: base64 }),
  })
  return result.ok ? { ok: true, value: result.value.has_avatar } : result
}

export async function deleteAvatar(): Promise<ApiResult<boolean>> {
  const result = await request<{ has_avatar: boolean }>('/api/me/avatar', {
    method: 'DELETE',
  })
  return result.ok ? { ok: true, value: result.value.has_avatar } : result
}

type RawWarning = {
  id: string
  reason: string
  created_at: string
  acknowledged: boolean
}

function toWarning(raw: RawWarning): Warning {
  return {
    id: raw.id,
    reason: raw.reason,
    createdAt: raw.created_at,
    acknowledged: raw.acknowledged,
  }
}

function banPath(username: string): string {
  return `/api/users/${encodeURIComponent(username)}/ban`
}

function ignorePath(username: string): string {
  return `/api/users/${encodeURIComponent(username)}/ignore`
}

export async function banUser(
  username: string,
  reason: string,
  days: number | null,
): Promise<ApiResult<Ban>> {
  return request<Ban>(banPath(username), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ reason, days }),
  })
}

export async function liftBan(username: string): Promise<ApiResult<void>> {
  return request<void>(banPath(username), { method: 'DELETE' })
}

export async function warnUser(username: string, reason: string): Promise<ApiResult<Warning>> {
  const result = await request<RawWarning>(
    `/api/users/${encodeURIComponent(username)}/warn`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason }),
    },
  )
  return result.ok ? { ok: true, value: toWarning(result.value) } : result
}

export async function getMyWarnings(): Promise<ApiResult<Warning[]>> {
  const result = await request<RawWarning[]>('/api/me/warnings', { method: 'GET' })
  return result.ok ? { ok: true, value: result.value.map(toWarning) } : result
}

export async function acknowledgeWarnings(): Promise<ApiResult<void>> {
  return request<void>('/api/me/warnings/acknowledge', { method: 'POST' })
}

export async function getIgnoreState(username: string): Promise<ApiResult<boolean>> {
  const result = await request<{ ignored: boolean }>(ignorePath(username), { method: 'GET' })
  return result.ok ? { ok: true, value: result.value.ignored } : result
}

export async function ignoreUser(username: string): Promise<ApiResult<boolean>> {
  const result = await request<{ ignored: boolean }>(ignorePath(username), { method: 'POST' })
  return result.ok ? { ok: true, value: result.value.ignored } : result
}

export async function stopIgnoring(username: string): Promise<ApiResult<boolean>> {
  const result = await request<{ ignored: boolean }>(ignorePath(username), {
    method: 'DELETE',
  })
  return result.ok ? { ok: true, value: result.value.ignored } : result
}

export async function promoteUser(username: string): Promise<ApiResult<User>> {
  return request<User>(`/api/users/${encodeURIComponent(username)}/promote`, {
    method: 'POST',
  })
}

export async function setUserRole(username: string, role: Role): Promise<ApiResult<User>> {
  return request<User>(`/api/users/${encodeURIComponent(username)}/role`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ role }),
  })
}

type RawAddressBlock = {
  addr: string
  reason: string
  blocked_at: string
  until: string | null
}

function toAddressBlock(raw: RawAddressBlock): AddressBlock {
  return {
    addr: raw.addr,
    reason: raw.reason,
    blockedAt: raw.blocked_at,
    until: raw.until,
  }
}

function blockPath(addr: string): string {
  return `/api/address-blocks/${encodeURIComponent(addr)}`
}

export async function listAddressBlocks(): Promise<ApiResult<AddressBlock[]>> {
  const result = await request<RawAddressBlock[]>('/api/address-blocks', { method: 'GET' })
  return result.ok ? { ok: true, value: result.value.map(toAddressBlock) } : result
}

export async function blockAddress(
  addr: string,
  reason: string,
  days: number | null,
): Promise<ApiResult<AddressBlock>> {
  const result = await request<RawAddressBlock>('/api/address-blocks', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ addr, reason, days }),
  })
  return result.ok ? { ok: true, value: toAddressBlock(result.value) } : result
}

export async function liftAddressBlock(addr: string): Promise<ApiResult<void>> {
  return request<void>(blockPath(addr), { method: 'DELETE' })
}

type RawAddressPost = {
  username: string
  addr: string
  client: string | null
  at: string
}

function toAddressPost(raw: RawAddressPost): AddressPost {
  return {
    username: raw.username,
    addr: raw.addr,
    client: raw.client,
    at: raw.at,
  }
}

function addressPath(addr: string): string {
  return `/api/addresses/${encodeURIComponent(addr)}`
}

export function listAddressPosts(
  addr: string,
  page?: number,
  size?: number,
): Promise<ApiResult<Paged<AddressPost>>> {
  return requestPage<RawAddressPost, AddressPost>(
    `${addressPath(addr)}/posts${pageQuery(page, size)}`,
    toAddressPost,
  )
}

export function removeAddressPosts(
  addr: string,
  hours: number,
  reason: string,
): Promise<ApiResult<{ removed: number }>> {
  return post<{ removed: number }>(`${addressPath(addr)}/remove-posts`, { hours, reason })
}

type RawReport = {
  id: string
  topic_id: string
  comment_id: string | null
  reporter_username: string
  kind: ReportKind
  reason: string
  created_at: string
}

function toReport(raw: RawReport): Report {
  return {
    id: raw.id,
    topicId: raw.topic_id,
    commentId: raw.comment_id,
    reporterUsername: raw.reporter_username,
    kind: raw.kind,
    reason: raw.reason,
    createdAt: raw.created_at,
  }
}

export function reportTopic(
  topicId: string,
  kind: ReportKind,
  reason: string,
): Promise<ApiResult<void>> {
  return post<void>(`/api/topics/${encodeURIComponent(topicId)}/report`, { kind, reason })
}

export function reportComment(
  topicId: string,
  commentId: string,
  kind: ReportKind,
  reason: string,
): Promise<ApiResult<void>> {
  return post<void>(
    `/api/topics/${encodeURIComponent(topicId)}/comments/${encodeURIComponent(
      commentId,
    )}/report`,
    { kind, reason },
  )
}

export function listReports(page?: number, size?: number): Promise<ApiResult<Paged<Report>>> {
  return requestPage<RawReport, Report>(`/api/reports${pageQuery(page, size)}`, toReport)
}

export function closeReport(id: string): Promise<ApiResult<void>> {
  return request<void>(`/api/reports/${encodeURIComponent(id)}/close`, { method: 'POST' })
}

export type MaintenanceReport = { blocked: number; dropped: number }

export function runMaintenance(): Promise<ApiResult<MaintenanceReport>> {
  return request<MaintenanceReport>('/api/maintenance/run', { method: 'POST' })
}
