<script setup lang="ts">
import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { postComment, deleteComment, type Comment } from '@/api/client'
import { renderMarkdown } from '@/lib/markdown'

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

function children(id: string | null) {
  return props.comments.filter((c) => c.parentId === id)
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
</script>

<template>
  <ul>
    <li v-for="comment in children(parentId)" :key="comment.id">
      <p><strong>{{ comment.authorUsername }}</strong></p>
      <p v-if="comment.deleted" class="removed">
        Removed by a moderator: {{ comment.deletedReason }}
      </p>
      <div v-else class="body" v-html="renderMarkdown(comment.body)"></div>

      <template v-if="!comment.deleted">
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
