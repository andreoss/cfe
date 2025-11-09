<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { getTopic, getComments, postComment, type Topic, type Comment } from '@/api/client'
import CommentThread from '@/components/CommentThread.vue'
import { renderMarkdown } from '@/lib/markdown'

const props = defineProps<{ id: string }>()
const auth = useAuthStore()

const topic = ref<Topic | null>(null)
const notFound = ref(false)
const comments = ref<Comment[]>([])
const newCommentDraft = ref('')
const formError = ref('')

async function load() {
  notFound.value = false
  topic.value = null
  const result = await getTopic(props.id)
  if (result.ok) {
    topic.value = result.value
  } else {
    notFound.value = true
    return
  }
  await loadComments()
}

async function loadComments() {
  const result = await getComments(props.id)
  if (result.ok) {
    comments.value = result.value
  }
}

watch(() => props.id, load, { immediate: true })

async function onPostComment() {
  formError.value = ''
  const result = await postComment(props.id, newCommentDraft.value, null)
  if (!result.ok) {
    formError.value = result.error
    return
  }
  newCommentDraft.value = ''
  await loadComments()
}
</script>

<template>
  <main>
    <p v-if="notFound">Topic not found.</p>
    <template v-else-if="topic">
      <h1>{{ topic.title }}</h1>
      <p>
        by <RouterLink :to="`/u/${topic.authorUsername}`">{{ topic.authorUsername }}</RouterLink>
        in <RouterLink :to="`/s/${topic.sectionSlug}`">{{ topic.sectionSlug }}</RouterLink>
      </p>
      <p v-if="topic.tags.length > 0">
        Tags:
        <RouterLink v-for="tag in topic.tags" :key="tag" :to="`/tag/${tag}`">{{ tag }}</RouterLink>
      </p>
      <div class="body" v-html="renderMarkdown(topic.body)"></div>

      <h2>Comments</h2>
      <CommentThread :comments="comments" :parent-id="null" :topic-id="id" :on-posted="loadComments" />
      <p v-if="comments.length === 0">No comments yet.</p>

      <form v-if="auth.currentUser" @submit.prevent="onPostComment">
        <label>
          Add a comment
          <textarea v-model="newCommentDraft" name="comment-body" rows="4"></textarea>
        </label>
        <p v-if="formError" role="alert">{{ formError }}</p>
        <button type="submit">Post comment</button>
      </form>
    </template>
  </main>
</template>
