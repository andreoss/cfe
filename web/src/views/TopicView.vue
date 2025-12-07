<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getTopic,
  getComments,
  postComment,
  deleteTopic,
  restoreTopic,
  editTopic,
  getBookmarkState,
  addBookmark,
  removeBookmark,
  getWatchState,
  watchTopic,
  unwatchTopic,
  getTopicReactions,
  reactToTopic,
  clearTopicReaction,
  getPoll,
  createPoll,
  votePoll,
  setPostscore,
  commitTopic,
  uncommitTopic,
  moveTopic,
  getGroups,
  reportTopic,
  publishTopic,
  setSticky,
  setOffFront,
  setResolved,
  DELETION_PENALTIES,
  DEFAULT_DELETION_PENALTY,
  type ReportKind,
  type Topic,
  type Comment,
  type PageInfo,
  type ReactionSummary,
  type Poll,
  type Group,
} from '@/api/client'
import CommentThread from '@/components/CommentThread.vue'
import PollPanel from '@/components/PollPanel.vue'
import ReactionBar from '@/components/ReactionBar.vue'
import UserAvatar from '@/components/UserAvatar.vue'
import { renderMarkdown } from '@/lib/markdown'
import { exactWhen, readableWhen } from '@/lib/when'
import { useDisplayStore } from '@/stores/display'
import {
  attachImage,
  getTopicImages,
  removeImage,
  topicImageUrl,
  type TopicImage,
} from '@/api/client'

const props = defineProps<{ id: string }>()
const auth = useAuthStore()
const display = useDisplayStore()

function shownWhen(iso: string) {
  return display.timeStyle === "exact" ? exactWhen(iso) : readableWhen(iso)
}

const topic = ref<Topic | null>(null)
const notFound = ref(false)
const comments = ref<Comment[]>([])
const commentPage = ref<PageInfo | null>(null)
const currentCommentPage = ref(1)
const newCommentDraft = ref('')
const formError = ref('')
const challengeNeeded = ref(false)
const challengeAnswer = ref('')
const deleting = ref(false)
const deleteReason = ref('')
const deletePenalty = ref(DEFAULT_DELETION_PENALTY)
const deleteError = ref('')
const restoreError = ref('')
const postscoreOpen = ref(false)
const postscoreError = ref('')

const postscoreOptions = [
  { value: -9999, label: 'Anyone' },
  { value: -50, label: 'Registered only' },
  { value: 50, label: 'Score 50' },
  { value: 100, label: 'Score 100' },
  { value: 200, label: 'Score 200' },
  { value: 300, label: 'Score 300' },
  { value: 400, label: 'Score 400' },
  { value: 500, label: 'Score 500' },
  { value: 9999, label: 'Moderators and author' },
  { value: 10000, label: 'Moderators only' },
  { value: 10001, label: 'No comments' },
]
const editing = ref(false)
const editTitle = ref('')
const editBody = ref('')
const editTags = ref('')
const editMinor = ref(false)
const editError = ref('')
const lifecycleError = ref('')
const placementError = ref('')
const bookmarked = ref(false)
const bookmarkError = ref('')
const watching = ref(false)
const watchError = ref('')
const reactions = ref<ReactionSummary | null>(null)
const poll = ref<Poll | null>(null)
const creatingPoll = ref(false)
const groups = ref<Group[]>([])
const moveTarget = ref('')
const moderationError = ref('')
const reporting = ref(false)
const reportKind = ref<ReportKind>('rule')
const reportReason = ref('')
const reportError = ref('')
const reportSent = ref(false)

const reportKinds: { value: ReportKind; label: string }[] = [
  { value: 'rule', label: 'Rule' },
  { value: 'spelling', label: 'Spelling' },
  { value: 'tag', label: 'Tag' },
  { value: 'group', label: 'Group' },
]

function startReport() {
  reportKind.value = 'rule'
  reportReason.value = ''
  reportError.value = ''
  reportSent.value = false
  reporting.value = true
}

async function onReport() {
  reportError.value = ''
  const result = await reportTopic(props.id, reportKind.value, reportReason.value)
  if (!result.ok) {
    reportError.value = result.error
    return
  }
  reportReason.value = ''
  reporting.value = false
  reportSent.value = true
}

async function loadGroups() {
  if (!auth.currentUser) return
  const result = await getGroups(topic.value?.sectionSlug ?? props.id)
  if (result.ok) groups.value = result.value
}

async function onCommit() {
  moderationError.value = ''
  const result = await commitTopic(props.id)
  if (result.ok) topic.value = result.value
  else moderationError.value = result.error
}

async function onUncommit() {
  moderationError.value = ''
  const result = await uncommitTopic(props.id)
  if (result.ok) topic.value = result.value
  else moderationError.value = result.error
}

async function onMove() {
  moderationError.value = ''
  if (!moveTarget.value) return
  const result = await moveTopic(props.id, moveTarget.value)
  if (result.ok) topic.value = result.value
  else moderationError.value = result.error
}
const pollQuestion = ref('')
const pollOptions = ref(['', '', ''])
const pollError = ref('')

const mayAddPoll = computed(
  () =>
    topic.value !== null &&
    !topic.value.deleted &&
    poll.value === null &&
    auth.currentUser !== null &&
    auth.currentUser.username === topic.value.authorUsername,
)

const mayEdit = computed(
  () =>
    topic.value !== null &&
    !topic.value.deleted &&
    auth.currentUser !== null &&
    (auth.currentUser.username === topic.value.authorUsername ||
      auth.currentUser.role === 'corrector' ||
      auth.currentUser.role === 'moderator'),
)

const isModerator = computed(() => auth.currentUser?.role === 'moderator')

const isAuthor = computed(
  () =>
    topic.value !== null &&
    auth.currentUser !== null &&
    auth.currentUser.username === topic.value.authorUsername,
)

const mayPublish = computed(() => topic.value !== null && topic.value.draft && isAuthor.value)

const images = ref<TopicImage[]>([])
const imageFile = ref<File | null>(null)
const imageError = ref('')

const mayAttach = computed(() => isAuthor.value || isModerator.value)

function mayRemoveImage(picture: TopicImage) {
  return isModerator.value || auth.currentUser?.username === picture.uploadedBy
}

function imageSrc(picture: TopicImage) {
  return topicImageUrl(props.id, picture.id)
}

function onImagePick(event: Event) {
  const input = event.target as HTMLInputElement
  imageFile.value = input.files?.[0] ?? null
}

function readImageBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const text = typeof reader.result === 'string' ? reader.result : ''
      const comma = text.indexOf(',')
      resolve(comma >= 0 ? text.slice(comma + 1) : text)
    }
    reader.onerror = () => reject(new Error('file could not be read'))
    reader.readAsDataURL(file)
  })
}

async function loadImages() {
  const result = await getTopicImages(props.id)
  images.value = result.ok ? result.value : []
}

async function onAttachImage() {
  imageError.value = ''
  const file = imageFile.value
  if (file === null) return
  const base64 = await readImageBase64(file)
  const result = await attachImage(props.id, base64)
  if (!result.ok) {
    imageError.value = result.error
    return
  }
  imageFile.value = null
  await loadImages()
}

async function onRemoveImage(picture: TopicImage) {
  imageError.value = ''
  const result = await removeImage(props.id, picture.id)
  if (!result.ok) {
    imageError.value = result.error
    return
  }
  await loadImages()
}

const mayResolve = computed(
  () => topic.value !== null && (isAuthor.value || isModerator.value),
)

async function onPublish() {
  lifecycleError.value = ''
  const result = await publishTopic(props.id)
  if (!result.ok) {
    lifecycleError.value = result.error
    return
  }
  topic.value = result.value
}

async function onToggleResolved() {
  lifecycleError.value = ''
  if (!topic.value) return
  const result = await setResolved(props.id, !topic.value.resolved)
  if (!result.ok) {
    lifecycleError.value = result.error
    return
  }
  topic.value = result.value
}

async function onToggleSticky() {
  placementError.value = ''
  if (!topic.value) return
  const result = await setSticky(props.id, !topic.value.sticky)
  if (!result.ok) {
    placementError.value = result.error
    return
  }
  topic.value = result.value
}

async function onToggleOffFront() {
  placementError.value = ''
  if (!topic.value) return
  const result = await setOffFront(props.id, !topic.value.offFront)
  if (!result.ok) {
    placementError.value = result.error
    return
  }
  topic.value = result.value
}

function startEdit() {
  if (!topic.value) return
  editTitle.value = topic.value.title
  editBody.value = topic.value.body
  editTags.value = topic.value.tags.join(', ')
  editMinor.value = false
  editError.value = ''
  editing.value = true
}

async function loadBookmarkState() {
  if (auth.currentUser === null) {
    bookmarked.value = false
    return
  }
  bookmarkError.value = ''
  const result = await getBookmarkState(props.id)
  if (result.ok) {
    bookmarked.value = result.value
  } else {
    bookmarkError.value = result.error
  }
}

async function loadWatchState() {
  if (auth.currentUser === null) {
    watching.value = false
    return
  }
  watchError.value = ''
  const asked = props.id
  const result = await getWatchState(asked)
  if (asked !== props.id) return
  if (result.ok) {
    watching.value = result.value
  } else {
    watchError.value = result.error
  }
}

async function onWatch() {
  watchError.value = ''
  const result = await watchTopic(props.id)
  if (!result.ok) {
    watchError.value = result.error
    return
  }
  watching.value = true
}

async function onUnwatch() {
  watchError.value = ''
  const result = await unwatchTopic(props.id)
  if (!result.ok) {
    watchError.value = result.error
    return
  }
  watching.value = false
}

async function loadReactions() {
  reactions.value = null
  const result = await getTopicReactions(props.id)
  if (result.ok) {
    reactions.value = result.value
  }
}

async function onReact(kind: string) {
  const result =
    reactions.value?.mine === kind
      ? await clearTopicReaction(props.id)
      : await reactToTopic(props.id, kind)
  if (result.ok) {
    reactions.value = result.value
  }
}

async function loadPoll() {
  poll.value = null
  creatingPoll.value = false
  pollError.value = ''
  const result = await getPoll(props.id)
  if (result.ok) {
    poll.value = result.value
  }
}

async function onVote(optionId: string) {
  const result = await votePoll(props.id, optionId)
  if (result.ok) {
    poll.value = result.value
  }
}

function startPollCreate() {
  pollQuestion.value = ''
  pollOptions.value = ['', '', '']
  pollError.value = ''
  creatingPoll.value = true
}

async function onCreatePoll() {
  pollError.value = ''
  const options = pollOptions.value.map((o) => o.trim()).filter((o) => o.length > 0)
  const result = await createPoll(props.id, pollQuestion.value, options)
  if (!result.ok) {
    pollError.value = result.error
    return
  }
  poll.value = result.value
  creatingPoll.value = false
}

async function load() {
  notFound.value = false
  topic.value = null
  currentCommentPage.value = 1
  reporting.value = false
  reportSent.value = false
  reportError.value = ''
  lifecycleError.value = ''
  placementError.value = ''
  restoreError.value = ''
  const result = await getTopic(props.id)
  if (result.ok) {
    topic.value = result.value
  } else {
    notFound.value = true
    return
  }
  await loadComments()
  await loadBookmarkState()
  await loadReactions()
  await loadPoll()
  if (auth.currentUser?.role === 'moderator') await loadGroups()
}

async function loadComments() {
  const result = await getComments(props.id, currentCommentPage.value)
  if (result.ok) {
    comments.value = result.value.items
    commentPage.value = result.value.page
  }
}

async function goToCommentPage(target: number) {
  currentCommentPage.value = target
  waiting.value = 0
  await loadComments()
}

const waiting = ref(0)
let watcher: ReturnType<typeof setInterval> | null = null

const POLL_MS = Number(import.meta.env.VITE_POLL_MS ?? 15000)

function waitingLabel() {
  return waiting.value === 1 ? 'Show 1 new comment' : `Show ${waiting.value} new comments`
}

async function lookForNew() {
  if (document.hidden) return
  const result = await getComments(props.id, currentCommentPage.value)
  if (!result.ok) return
  const known = new Set(comments.value.map((c) => c.id))
  const fresh = result.value.items.filter((c) => !known.has(c.id))
  waiting.value = fresh.length
}

async function showWaiting() {
  waiting.value = 0
  await loadComments()
}

async function onPosted() {
  waiting.value = 0
  await loadComments()
}

function watchForNew() {
  if (watcher !== null) clearInterval(watcher)
  if (POLL_MS <= 0) return
  watcher = setInterval(() => {
    void lookForNew()
  }, POLL_MS)
}

onUnmounted(() => {
  if (watcher !== null) clearInterval(watcher)
})

function previousComments() {
  if (commentPage.value) void goToCommentPage(commentPage.value.number - 1)
}

function nextComments() {
  if (commentPage.value) void goToCommentPage(commentPage.value.number + 1)
}

watch(() => props.id, load, { immediate: true })
watch([() => props.id, () => auth.currentUser], loadWatchState, { immediate: true })
watch([() => props.id, () => auth.currentUser], loadBookmarkState, { immediate: true })
watch(() => props.id, loadImages, { immediate: true })
watch(
  () => props.id,
  () => {
    waiting.value = 0
    watchForNew()
  },
  { immediate: true },
)

async function onPostComment() {
  formError.value = ''
  const result = await postComment(props.id, newCommentDraft.value, null, challengeAnswer.value)
  if (!result.ok) {
    if (result.status === 428 && !challengeNeeded.value) {
      challengeNeeded.value = true
      challengeAnswer.value = ''
      return
    }
    formError.value = result.error
    return
  }
  newCommentDraft.value = ''
  challengeNeeded.value = false
  challengeAnswer.value = ''
  await loadComments()
}

async function onDelete() {
  deleteError.value = ''
  const result = await deleteTopic(props.id, deleteReason.value, deletePenalty.value)
  if (!result.ok) {
    deleteError.value = result.error
    return
  }
  topic.value = result.value
  deleting.value = false
}

async function onRestore() {
  restoreError.value = ''
  const result = await restoreTopic(props.id)
  if (!result.ok) {
    restoreError.value = result.error
    return
  }
  topic.value = result.value
}

async function onSetPostscore(value: number) {
  postscoreError.value = ''
  const result = await setPostscore(props.id, value)
  if (!result.ok) {
    postscoreError.value = result.error
    return
  }
  topic.value = result.value
  postscoreOpen.value = false
}

async function onEdit() {
  editError.value = ''
  const tags = editTags.value
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t.length > 0)
  const result = await editTopic(
    props.id,
    editTitle.value,
    editBody.value,
    tags,
    editMinor.value,
  )
  if (!result.ok) {
    editError.value = result.error
    return
  }
  topic.value = result.value
  editing.value = false
}

async function onSave() {
  bookmarkError.value = ''
  const result = await addBookmark(props.id)
  if (!result.ok) {
    bookmarkError.value = result.error
    return
  }
  bookmarked.value = result.value
}

async function onUnsave() {
  bookmarkError.value = ''
  const result = await removeBookmark(props.id)
  if (!result.ok) {
    bookmarkError.value = result.error
    return
  }
  bookmarked.value = result.value
}
</script>

<template>
  <main>
    <template v-if="notFound">
      <h1>There is nothing here</h1>
      <p>
        This subject is not here. It may have been removed, or the address may have a typo in
        it. From here you can <RouterLink to="/">go back to the sections</RouterLink> or
        <RouterLink to="/search">search for what you wanted</RouterLink>.
      </p>
    </template>
    <template v-else-if="topic">
      <h1>{{ topic.title }}</h1>
      <p class="byline">
        <time :datetime="topic.createdAt" :title="exactWhen(topic.createdAt)">{{
          shownWhen(topic.createdAt)
        }}</time>
        by <UserAvatar :username="topic.authorUsername" />
        <RouterLink :to="`/u/${topic.authorUsername}`">{{ topic.authorUsername }}</RouterLink>
        in <RouterLink :to="`/s/${topic.sectionSlug}`">{{ topic.sectionSlug }}</RouterLink>
        <template v-if="topic.groupSlug">
          / <RouterLink :to="`/s/${topic.sectionSlug}/g/${topic.groupSlug}`">{{ topic.groupSlug }}</RouterLink>
        </template>
      </p>
      <p v-if="topic.tags.length > 0" class="tag-line">
        Tags:
        <RouterLink v-for="tag in topic.tags" :key="tag" :to="`/tag/${tag}`">{{ tag }}</RouterLink>
      </p>

      <p v-if="topic.pending" role="status" class="queued">Awaiting moderation.</p>

      <p v-if="topic.draft" class="draft">Draft</p>
      <p v-if="topic.resolved" class="resolved">Resolved</p>

      <template v-if="mayPublish">
        <button type="button" @click="onPublish">Publish draft</button>
      </template>
      <template v-if="mayResolve">
        <button v-if="!topic.resolved" type="button" @click="onToggleResolved">Mark resolved</button>
        <button v-else type="button" @click="onToggleResolved">Mark unresolved</button>
      </template>
      <p v-if="lifecycleError" role="alert">{{ lifecycleError }}</p>

      <p v-if="topic.deleted" class="removed">Removed by a moderator: {{ topic.deletedReason }}</p>
      <div v-else class="body" v-html="renderMarkdown(topic.body)"></div>

      <div v-if="images.length > 0" class="attachments">
        <figure v-for="picture in images" :key="picture.id">
          <img
            class="attachment"
            :src="imageSrc(picture)"
            :alt="`attached by ${picture.uploadedBy}`"
          />
          <figcaption v-if="mayRemoveImage(picture)">
            <button type="button" @click="onRemoveImage(picture)">Remove image</button>
          </figcaption>
        </figure>
      </div>

      <div v-if="mayAttach && !topic.deleted" class="attach">
        <label>
          Image
          <input name="image-file" type="file" accept="image/*" @change="onImagePick" />
        </label>
        <button type="button" @click="onAttachImage">Attach image</button>
        <p v-if="imageError" role="alert">{{ imageError }}</p>
      </div>
      <p v-if="topic.edited && !topic.deleted" class="edited">(edited)</p>
      <p v-if="topic.edited && !topic.deleted">
        <RouterLink :to="`/t/${id}/history`">History</RouterLink>
      </p>

      <template v-if="isModerator && topic.deleted">
        <button type="button" @click="onRestore">Restore topic</button>
        <p v-if="restoreError" role="alert">{{ restoreError }}</p>
      </template>

      <PollPanel :poll="poll" :can-vote="auth.currentUser !== null" :on-vote="onVote" />

      <template v-if="mayAddPoll">
        <button v-if="!creatingPoll" type="button" @click="startPollCreate">Add poll</button>
        <form v-else @submit.prevent="onCreatePoll">
          <label>
            Question
            <input v-model="pollQuestion" name="poll-question" type="text" />
          </label>
          <label>
            Option 1
            <input v-model="pollOptions[0]" name="poll-option-1" type="text" />
          </label>
          <label>
            Option 2
            <input v-model="pollOptions[1]" name="poll-option-2" type="text" />
          </label>
          <label>
            Option 3
            <input v-model="pollOptions[2]" name="poll-option-3" type="text" />
          </label>
          <p v-if="pollError" role="alert">{{ pollError }}</p>
          <button type="submit">Create poll</button>
          <button type="button" @click="creatingPoll = false">Cancel</button>
        </form>
      </template>

      <ReactionBar
        v-if="!topic.deleted"
        :summary="reactions"
        :disabled="auth.currentUser === null"
        :on-pick="onReact"
      />

      <template v-if="mayEdit">
        <button v-if="!editing" type="button" @click="startEdit">Edit topic</button>
        <form v-else @submit.prevent="onEdit">
          <label>
            Title
            <input v-model="editTitle" name="edit-title" type="text" />
          </label>
          <label>
            Body
            <textarea v-model="editBody" name="edit-body" rows="4"></textarea>
          </label>
          <label>
            Tags
            <input v-model="editTags" name="edit-tags" type="text" />
          </label>
          <label>
            <input v-model="editMinor" name="edit-minor" type="checkbox" />
            Minor edit
          </label>
          <p v-if="editError" role="alert">{{ editError }}</p>
          <button type="submit">Save changes</button>
          <button type="button" @click="editing = false">Cancel</button>
        </form>
      </template>

      <p v-if="auth.currentUser?.role === 'moderator'">Open reports: {{ topic.openReports }}</p>

      <template v-if="auth.currentUser?.role === 'moderator' && !topic.deleted">
        <button v-if="!deleting" type="button" @click="deleting = true">Delete topic</button>
        <form v-else @submit.prevent="onDelete">
          <label>
            Reason
            <input v-model="deleteReason" name="delete-reason" type="text" />
          </label>
          <label>
            Penalty
            <select v-model="deletePenalty" name="penalty">
              <option v-for="p in DELETION_PENALTIES" :key="p.value" :value="p.value">
                {{ p.label }}
              </option>
            </select>
          </label>
          <p v-if="deleteError" role="alert">{{ deleteError }}</p>
          <button type="submit">Confirm delete</button>
          <button type="button" @click="deleting = false">Cancel</button>
        </form>
        <details v-if="!deleting && !editing">
          <summary>Comment restrictions</summary>
          <select :value="topic.postscore" name="postscore" @change="onSetPostscore(Number(($event.target as HTMLSelectElement).value))">
            <option v-for="opt in postscoreOptions" :key="opt.value" :value="opt.value" :selected="topic.postscore === opt.value">{{ opt.label }}</option>
          </select>
          <p v-if="postscoreError" role="alert">{{ postscoreError }}</p>
        </details>
        <details v-if="!deleting && !editing">
          <summary>Premoderation</summary>
          <button v-if="topic.pending" type="button" @click="onCommit">Commit topic</button>
          <button v-else type="button" @click="onUncommit">Return to queue</button>
          <label>
            Move to group
            <select v-model="moveTarget" name="move-group">
              <option value="">— none —</option>
              <option v-for="g in groups" :key="g.id" :value="g.slug">{{ g.name }}</option>
            </select>
          </label>
          <button type="button" :disabled="!moveTarget" @click="onMove">Move</button>
          <p v-if="moderationError" role="alert">{{ moderationError }}</p>
        </details>
        <details v-if="!deleting && !editing">
          <summary>Placement</summary>
          <button v-if="!topic.sticky" type="button" @click="onToggleSticky">Make sticky</button>
          <button v-else type="button" @click="onToggleSticky">Unstick</button>
          <button v-if="!topic.offFront" type="button" @click="onToggleOffFront">Hide from front</button>
          <button v-else type="button" @click="onToggleOffFront">Show on front</button>
          <p v-if="placementError" role="alert">{{ placementError }}</p>
        </details>
      </template>

      <template v-if="auth.currentUser && !topic.deleted">
        <button v-if="!bookmarked" type="button" @click="onSave">Save topic</button>
        <button v-else type="button" @click="onUnsave">Unsave topic</button>
        <p v-if="bookmarkError" role="alert">{{ bookmarkError }}</p>
      </template>

      <template v-if="auth.currentUser">
        <button v-if="!watching" type="button" @click="onWatch">Watch topic</button>
        <button v-else type="button" @click="onUnwatch">Stop watching</button>
        <p v-if="watchError" role="alert">{{ watchError }}</p>
      </template>

      <template v-if="auth.currentUser">
        <button v-if="!reporting" type="button" @click="startReport">Report</button>
        <form v-else @submit.prevent="onReport">
          <label>
            Kind
            <select v-model="reportKind" name="report-kind">
              <option v-for="k in reportKinds" :key="k.value" :value="k.value">{{ k.label }}</option>
            </select>
          </label>
          <label>
            Reason
            <input v-model="reportReason" name="report-reason" type="text" />
          </label>
          <p v-if="reportError" role="alert">{{ reportError }}</p>
          <button type="submit">Send report</button>
          <button type="button" @click="reporting = false">Cancel</button>
        </form>
        <p v-if="reportSent" role="status">Report sent.</p>
      </template>

      <h2>Comments</h2>
      <button v-if="waiting > 0" type="button" class="waiting" @click="showWaiting">
        {{ waitingLabel() }}
      </button>
      <CommentThread :comments="comments" :parent-id="null" :topic-id="id" :on-posted="onPosted" />
      <p v-if="comments.length === 0">No comments yet.</p>

      <nav v-if="commentPage && commentPage.totalPages > 1">
        <button type="button" :disabled="!commentPage.hasPrevious" @click="previousComments">Previous</button>
        <span>Page {{ commentPage.number }} of {{ commentPage.totalPages }}</span>
        <button type="button" :disabled="!commentPage.hasNext" @click="nextComments">Next</button>
      </nav>

      <form v-if="auth.currentUser" @submit.prevent="onPostComment">
        <label>
          Add a comment
          <textarea v-model="newCommentDraft" name="comment-body" rows="4"></textarea>
        </label>
        <template v-if="challengeNeeded">
          <p role="status">Answer the challenge to continue.</p>
          <label>
            Challenge
            <input v-model="challengeAnswer" name="comment-challenge-answer" type="text" />
          </label>
        </template>
        <p v-if="formError" role="alert">{{ formError }}</p>
        <button type="submit">Post comment</button>
      </form>
    </template>
  </main>
</template>

<style scoped>
h1 {
  max-width: var(--reading);
  margin-bottom: 0;
}

.waiting {
  display: block;
  margin: var(--gap-3) 0;
  font: inherit;
  font-weight: 600;
}

.attachments {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gap-3);
  max-width: var(--reading);
}

.attachments figure {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--gap-1);
}

.attachment {
  max-width: 100%;
  max-height: 24rem;
  height: auto;
  border-radius: var(--round);
  border: 1px solid var(--edge-soft);
  background: var(--ground-sunk);
}

.attach {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: var(--gap-3);
  max-width: var(--reading);
}

.attach label {
  flex: 1 1 14rem;
}

.attach p[role="alert"] {
  flex-basis: 100%;
}

.byline time {
  color: var(--ink-faint);
  font-size: var(--step-tiny);
  white-space: nowrap;
}

.byline,
.tag-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--gap-1) var(--gap-2);
  font-size: var(--step-small);
  color: var(--ink-soft);
}

.byline a,
.tag-line a {
  font-weight: 550;
  text-decoration: none;
}

.byline a:hover,
.tag-line a:hover {
  text-decoration: underline;
}

.byline .avatar {
  display: inline-block;
  width: 1.75rem;
  height: 1.75rem;
}

.tag-line a {
  padding: 0.05rem var(--gap-2);
  border-radius: 999px;
  background: var(--ground-sunk);
  color: var(--ink-soft);
  font-size: var(--step-tiny);
}

.tag-line a:hover {
  background: var(--accent-soft);
  color: var(--accent);
  text-decoration: none;
}

.queued,
.draft,
.resolved {
  align-self: flex-start;
  padding: var(--gap-1) var(--gap-3);
  border-radius: var(--round);
  font-size: var(--step-small);
  font-weight: 550;
}

.queued {
  background: var(--warn-soft);
  color: var(--ink-soft);
}

.draft {
  background: var(--ground-sunk);
  color: var(--ink-soft);
}

.resolved {
  background: var(--good-soft);
  color: var(--good);
}

.body {
  padding: var(--gap-5);
  border: 1px solid var(--edge-soft);
  border-radius: var(--round-large);
  background: var(--ground);
  box-shadow: var(--shadow);
  font-size: var(--step-1);
  line-height: 1.7;
}

.body :deep(h1),
.body :deep(h2),
.body :deep(h3) {
  margin-top: var(--gap-5);
  font-size: var(--step-2);
}

.body :deep(blockquote) {
  padding-left: var(--gap-4);
  border-left: 3px solid var(--edge);
  color: var(--ink-soft);
}

.body :deep(img) {
  border-radius: var(--round);
}

.body :deep(li + li) {
  margin-top: var(--gap-2);
}

p.removed {
  max-width: var(--reading);
  padding: var(--gap-3) var(--gap-4);
  border-radius: var(--round-large);
  background: var(--ground-sunk);
  color: var(--ink-faint);
  font-family: inherit;
  font-size: var(--step-small);
  white-space: normal;
}

.edited {
  margin-top: calc(var(--gap-3) * -1);
  color: var(--ink-faint);
  font-size: var(--step-tiny);
}

main > button,
main > details,
main > nav {
  align-self: flex-start;
}

details {
  max-width: var(--reading);
}

summary {
  cursor: pointer;
  font-size: var(--step-small);
  font-weight: 550;
  color: var(--ink-soft);
}

h2 {
  max-width: var(--reading);
  margin-top: var(--gap-4);
  padding-top: var(--gap-5);
  border-top: 1px solid var(--edge);
}

main > nav {
  display: flex;
  align-items: stretch;
  border: 1px solid var(--edge);
  border-radius: var(--round);
  background: var(--ground);
}

main > nav button {
  border: 0;
  border-radius: 0;
  background: transparent;
  padding: var(--gap-2) var(--gap-4);
}

main > nav span {
  display: flex;
  align-items: center;
  padding: 0 var(--gap-4);
  border-left: 1px solid var(--edge-soft);
  border-right: 1px solid var(--edge-soft);
  font-size: var(--step-small);
  color: var(--ink-soft);
  white-space: nowrap;
}

@media (max-width: 40rem) {
  .body {
    padding: var(--gap-4);
  }

  main > nav {
    align-self: stretch;
  }

  main > nav span {
    flex: 1;
    justify-content: center;
  }
}
</style>
