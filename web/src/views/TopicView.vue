<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import {
  getTopic,
  getComments,
  postComment,
  deleteTopic,
  editTopic,
  type Topic,
  type Comment,
} from '@/api/client'
import CommentThread from '@/components/CommentThread.vue'
import { renderMarkdown } from '@/lib/markdown'

const props = defineProps<{ id: string }>()
const auth = useAuthStore()

const topic = ref<Topic | null>(null)
const notFound = ref(false)
const comments = ref<Comment[]>([])
const newCommentDraft = ref('')
const formError = ref('')
const deleting = ref(false)
const deleteReason = ref('')
const deleteError = ref('')
const editing = ref(false)
const editTitle = ref('')
const editBody = ref('')
const editTags = ref('')
const editError = ref('')

const mayEdit = computed(
  () =>
    topic.value !== null &&
    !topic.value.deleted &&
    auth.currentUser !== null &&
    (auth.currentUser.username === topic.value.authorUsername ||
      auth.currentUser.role === 'moderator'),
)

function startEdit() {
  if (!topic.value) return
  editTitle.value = topic.value.title
  editBody.value = topic.value.body
  editTags.value = topic.value.tags.join(', ')
  editError.value = ''
  editing.value = true
}

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

async function onDelete() {
  deleteError.value = ''
  const result = await deleteTopic(props.id, deleteReason.value)
  if (!result.ok) {
    deleteError.value = result.error
    return
  }
  topic.value = result.value
  deleting.value = false
}

async function onEdit() {
  editError.value = ''
  const tags = editTags.value
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t.length > 0)
  const result = await editTopic(props.id, editTitle.value, editBody.value, tags)
  if (!result.ok) {
    editError.value = result.error
    return
  }
  topic.value = result.value
  editing.value = false
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

      <p v-if="topic.deleted" class="removed">Removed by a moderator: {{ topic.deletedReason }}</p>
      <div v-else class="body" v-html="renderMarkdown(topic.body)"></div>
      <p v-if="topic.edited && !topic.deleted" class="edited">(edited)</p>

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
          <p v-if="editError" role="alert">{{ editError }}</p>
          <button type="submit">Save changes</button>
          <button type="button" @click="editing = false">Cancel</button>
        </form>
      </template>

      <template v-if="auth.currentUser?.role === 'moderator' && !topic.deleted">
        <button v-if="!deleting" type="button" @click="deleting = true">Delete topic</button>
        <form v-else @submit.prevent="onDelete">
          <label>
            Reason
            <input v-model="deleteReason" name="delete-reason" type="text" />
          </label>
          <p v-if="deleteError" role="alert">{{ deleteError }}</p>
          <button type="submit">Confirm delete</button>
          <button type="button" @click="deleting = false">Cancel</button>
        </form>
      </template>

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
