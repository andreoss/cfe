<script setup lang="ts">
import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { postComment, type Comment } from '@/api/client'
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
</script>

<template>
  <ul>
    <li v-for="comment in children(parentId)" :key="comment.id">
      <p><strong>{{ comment.authorUsername }}</strong></p>
      <div class="body" v-html="renderMarkdown(comment.body)"></div>
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
      <CommentThread
        :comments="comments"
        :parent-id="comment.id"
        :topic-id="topicId"
        :on-posted="onPosted"
      />
    </li>
  </ul>
</template>
