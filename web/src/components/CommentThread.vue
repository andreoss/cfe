<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  postComment,
  deleteComment,
  restoreComment,
  editComment,
  getCommentReactions,
  reactToComment,
  clearCommentReaction,
  reportComment,
  DELETION_PENALTIES,
  DEFAULT_DELETION_PENALTY,
  type Comment,
  type ReactionSummary,
  type ReportKind,
} from '@/api/client'
import { renderMarkdown } from '@/lib/markdown'
import { exactWhen, readableWhen } from '@/lib/when'
import { useDisplayStore } from '@/stores/display'
import ReactionBar from '@/components/ReactionBar.vue'
import UserAvatar from '@/components/UserAvatar.vue'

const props = defineProps<{
  comments: Comment[]
  parentId: string | null
  topicId: string
  onPosted: () => void
}>()

const display = useDisplayStore()

function shownWhen(iso: string) {
  return display.timeStyle === "exact" ? exactWhen(iso) : readableWhen(iso)
}

const auth = useAuthStore()
const replyingTo = ref<string | null>(null)
const draft = ref('')
const formError = ref('')
const deletingId = ref<string | null>(null)
const deleteReason = ref('')
const deletePenalty = ref(DEFAULT_DELETION_PENALTY)
const deleteError = ref('')
const restoreErrorId = ref<string | null>(null)
const restoreError = ref('')
const editingId = ref<string | null>(null)
const editDraft = ref('')
const editError = ref('')
const reportingId = ref<string | null>(null)
const reportKind = ref<ReportKind>('rule')
const reportReason = ref('')
const reportError = ref('')
const reportedIds = ref<string[]>([])

const reportKinds: { value: ReportKind; label: string }[] = [
  { value: 'rule', label: 'Rule' },
  { value: 'spelling', label: 'Spelling' },
  { value: 'tag', label: 'Tag' },
  { value: 'group', label: 'Group' },
]

function startReport(commentId: string) {
  reportingId.value = commentId
  reportKind.value = 'rule'
  reportReason.value = ''
  reportError.value = ''
  reportedIds.value = reportedIds.value.filter((id) => id !== commentId)
}

async function onReport(commentId: string) {
  reportError.value = ''
  const result = await reportComment(
    props.topicId,
    commentId,
    reportKind.value,
    reportReason.value,
  )
  if (!result.ok) {
    reportError.value = result.error
    return
  }
  reportingId.value = null
  reportReason.value = ''
  reportedIds.value = [...reportedIds.value, commentId]
}

const summaries = ref<Record<string, ReactionSummary>>({})
const requested = new Set<string>()

const collapsed = ref<Set<string>>(new Set())

function toggleCollapsed(id: string) {
  const next = new Set(collapsed.value)
  if (next.has(id)) {
    next.delete(id)
  } else {
    next.add(id)
  }
  collapsed.value = next
}

function foldLabel(id: string) {
  const count = replyCount(id)
  return count === 1 ? 'Show 1 reply' : `Show ${count} replies`
}

function replyCount(id: string) {
  return props.comments.filter((c) => c.parentId === id).length
}

function children(id: string | null) {
  return props.comments.filter((c) => c.parentId === id)
}

const reactable = computed(() =>
  children(props.parentId).filter((c) => !c.deleted && !c.ignored),
)

async function loadSummary(commentId: string) {
  if (requested.has(commentId)) return
  requested.add(commentId)
  const result = await getCommentReactions(props.topicId, commentId)
  if (result.ok) {
    summaries.value = { ...summaries.value, [commentId]: result.value }
  } else {
    requested.delete(commentId)
  }
}

watch(
  reactable,
  (list) => {
    for (const comment of list) {
      void loadSummary(comment.id)
    }
  },
  { immediate: true },
)

async function onReact(commentId: string, kind: string) {
  const current = summaries.value[commentId]
  const result =
    current?.mine === kind
      ? await clearCommentReaction(props.topicId, commentId)
      : await reactToComment(props.topicId, commentId, kind)
  if (result.ok) {
    summaries.value = { ...summaries.value, [commentId]: result.value }
  }
}

function mayEdit(comment: Comment) {
  const user = auth.currentUser
  return (
    user !== null &&
    (user.username === comment.authorUsername ||
      user.role === 'corrector' ||
      user.role === 'moderator')
  )
}

function startEdit(comment: Comment) {
  editingId.value = comment.id
  editDraft.value = comment.body
  editError.value = ''
}

async function onReply(parentId: string) {
  formError.value = ''
  const result = await postComment(props.topicId, draft.value, parentId)
  if (!result.ok) {
    formError.value = result.error
    return
  }
  draft.value = ''
  replyingTo.value = null
  props.onPosted()
}

async function onDelete(commentId: string) {
  deleteError.value = ''
  const result = await deleteComment(
    props.topicId,
    commentId,
    deleteReason.value,
    deletePenalty.value,
  )
  if (!result.ok) {
    deleteError.value = result.error
    return
  }
  deletingId.value = null
  deleteReason.value = ''
  deletePenalty.value = DEFAULT_DELETION_PENALTY
  props.onPosted()
}

async function onRestore(commentId: string) {
  restoreErrorId.value = null
  restoreError.value = ''
  const result = await restoreComment(props.topicId, commentId)
  if (!result.ok) {
    restoreErrorId.value = commentId
    restoreError.value = result.error
    return
  }
  props.onPosted()
}

async function onEdit(commentId: string) {
  editError.value = ''
  const result = await editComment(props.topicId, commentId, editDraft.value)
  if (!result.ok) {
    editError.value = result.error
    return
  }
  editingId.value = null
  editDraft.value = ''
  props.onPosted()
}
</script>

<template>
  <ul>
    <li v-for="comment in children(parentId)" :key="comment.id">
      <p class="byline">
        <UserAvatar :username="comment.authorUsername" />
        <strong>{{ comment.authorUsername }}</strong>
        <time :datetime="comment.createdAt" :title="exactWhen(comment.createdAt)">{{
          shownWhen(comment.createdAt)
        }}</time>
        <button
          v-if="replyCount(comment.id) > 0"
          type="button"
          class="fold"
          :aria-expanded="!collapsed.has(comment.id)"
          @click="toggleCollapsed(comment.id)"
        >
          {{ collapsed.has(comment.id) ? foldLabel(comment.id) : 'Hide replies' }}
        </button>
      </p>
      <p v-if="comment.deleted" class="removed">
        Removed by a moderator: {{ comment.deletedReason }}
      </p>
      <p v-else-if="comment.ignored" class="ignored">Hidden — you ignore this author.</p>
      <div v-else class="body" v-html="renderMarkdown(comment.body)"></div>
      <p v-if="comment.edited && !comment.deleted && !comment.ignored" class="edited">
        (edited)
      </p>

      <template v-if="auth.currentUser?.role === 'moderator' && comment.deleted">
        <button type="button" @click="onRestore(comment.id)">Restore comment</button>
        <p v-if="restoreErrorId === comment.id && restoreError" role="alert">
          {{ restoreError }}
        </p>
      </template>

      <template v-if="!comment.deleted && !comment.ignored">
        <ReactionBar
          :summary="summaries[comment.id] ?? null"
          :disabled="auth.currentUser === null"
          :on-pick="(kind: string) => onReact(comment.id, kind)"
        />

        <template v-if="mayEdit(comment)">
          <button
            v-if="editingId !== comment.id"
            type="button"
            @click="startEdit(comment)"
          >
            Edit comment
          </button>
          <form v-else @submit.prevent="onEdit(comment.id)">
            <textarea v-model="editDraft" name="edit-body" rows="3"></textarea>
            <p v-if="editError" role="alert">{{ editError }}</p>
            <button type="submit">Save comment</button>
            <button type="button" @click="editingId = null">Cancel</button>
          </form>
        </template>

        <button
          v-if="auth.currentUser"
          type="button"
          @click="replyingTo = replyingTo === comment.id ? null : comment.id"
        >
          Reply
        </button>
        <form v-if="replyingTo === comment.id" @submit.prevent="onReply(comment.id)">
          <textarea v-model="draft" name="reply-body" rows="3"></textarea>
          <p v-if="formError" role="alert">{{ formError }}</p>
          <button type="submit">Post reply</button>
        </form>

        <template v-if="auth.currentUser">
          <button
            v-if="reportingId !== comment.id"
            type="button"
            @click="startReport(comment.id)"
          >
            Report comment
          </button>
          <form v-else @submit.prevent="onReport(comment.id)">
            <label>
              Kind
              <select v-model="reportKind" name="comment-report-kind">
                <option v-for="k in reportKinds" :key="k.value" :value="k.value">
                  {{ k.label }}
                </option>
              </select>
            </label>
            <label>
              Reason
              <input v-model="reportReason" name="comment-report-reason" type="text" />
            </label>
            <p v-if="reportError" role="alert">{{ reportError }}</p>
            <button type="submit">Send comment report</button>
            <button type="button" @click="reportingId = null">Cancel</button>
          </form>
          <p v-if="reportedIds.includes(comment.id)" role="status">Report sent.</p>
        </template>

        <template v-if="auth.currentUser?.role === 'moderator'">
          <button
            v-if="deletingId !== comment.id"
            type="button"
            @click="deletingId = comment.id"
          >
            Delete comment
          </button>
          <form v-else @submit.prevent="onDelete(comment.id)">
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
            <button type="button" @click="deletingId = null">Cancel</button>
          </form>
        </template>
      </template>

      <CommentThread
        v-if="!collapsed.has(comment.id)"
        :comments="comments"
        :parent-id="comment.id"
        :topic-id="topicId"
        :on-posted="onPosted"
      />
    </li>
  </ul>
</template>

<style scoped>
ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--gap-3);
}

li > ul {
  margin-top: var(--gap-4);
  padding-left: var(--gap-4);
  border-left: 2px solid var(--edge);
  gap: var(--gap-4);
}

li > ul:empty {
  display: none;
}

li > ul:not(:has(> li)) {
  display: none;
}

ul ul ul ul ul ul {
  padding-left: var(--gap-2);
}

ul ul ul ul ul ul ul ul ul {
  padding-left: var(--gap-1);
  border-left-width: 1px;
}

li li {
  padding-top: var(--gap-1);
}

.byline time {
  color: var(--ink-faint);
  font-size: var(--step-tiny);
  white-space: nowrap;
}

.byline .fold {
  margin-left: auto;
  padding: 0.1rem 0.5rem;
  border-color: transparent;
  background: transparent;
  color: var(--ink-faint);
  font-size: var(--step-tiny);
  font-weight: 550;
}

.byline .fold:hover {
  background: var(--ground-sunk);
  color: var(--ink);
}

.byline {
  display: flex;
  align-items: center;
  gap: var(--gap-2);
  font-size: var(--step-small);
  color: var(--ink-soft);
}

.byline .avatar {
  width: 2rem;
  height: 2rem;
  flex: none;
}

.byline strong {
  color: var(--ink);
  font-weight: 650;
}

.body {
  margin-top: var(--gap-2);
  line-height: 1.65;
}

.body :deep(blockquote) {
  padding-left: var(--gap-3);
  border-left: 3px solid var(--edge-soft);
  color: var(--ink-soft);
}

.removed,
.ignored {
  display: block;
  max-width: var(--reading);
  margin-top: var(--gap-2);
  padding: var(--gap-2) var(--gap-3);
  border-radius: var(--round);
  background: var(--ground-sunk);
  color: var(--ink-faint);
  font-family: inherit;
  font-size: var(--step-small);
  white-space: normal;
}

.edited {
  display: block;
  margin-top: var(--gap-1);
  color: var(--ink-faint);
  font-size: var(--step-tiny);
}

li > button {
  margin-top: var(--gap-2);
  margin-right: var(--gap-1);
  padding: 0.2rem 0.6rem;
  border-color: transparent;
  background: transparent;
  color: var(--ink-soft);
  font-size: var(--step-tiny);
}

li > button:hover:not(:disabled) {
  background: var(--ground-sunk);
  color: var(--ink);
}

li > form {
  max-width: var(--reading);
  margin-top: var(--gap-3);
  background: var(--ground-soft);
}

li > p[role='status'] {
  margin-top: var(--gap-2);
  color: var(--ink-faint);
  font-size: var(--step-small);
}

li > p[role='alert'] {
  margin-top: var(--gap-2);
  max-width: var(--reading);
}

@media (max-width: 40rem) {
  li > ul {
    padding-left: var(--gap-3);
  }

  ul ul ul ul ul {
    padding-left: var(--gap-1);
  }
}
</style>
