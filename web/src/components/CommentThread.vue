<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  postComment,
  deleteComment,
  editComment,
  getCommentReactions,
  reactToComment,
  clearCommentReaction,
  type Comment,
  type ReactionSummary,
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
const deleteError = ref('')
const editingId = ref<string | null>(null)
const editDraft = ref('')
const editError = ref('')

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
    user !== null && (user.username === comment.authorUsername || user.role === 'moderator')
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
  const result = await deleteComment(props.topicId, commentId, deleteReason.value)
  if (!result.ok) {
    deleteError.value = result.error
    return
  }
  deletingId.value = null
  deleteReason.value = ''
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
