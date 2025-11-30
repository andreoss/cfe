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
import ReactionBar from '@/components/ReactionBar.vue'
import UserAvatar from '@/components/UserAvatar.vue'

const props = defineProps<{
  comments: Comment[]
  parentId: string | null
  topicId: string
  onPosted: () => void
}>()

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
      <p>
        <UserAvatar :username="comment.authorUsername" />
        <strong>{{ comment.authorUsername }}</strong>
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
        :comments="comments"
        :parent-id="comment.id"
        :topic-id="topicId"
        :on-posted="onPosted"
      />
    </li>
  </ul>
</template>
