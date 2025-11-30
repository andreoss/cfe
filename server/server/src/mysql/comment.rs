use crate::mysql::topic::{revision, stored_penalty};
use app::CommentRepository;
use domain::{Body, Comment, CommentId, Deletion, Page, Penalty, Reason, TopicId, UserId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlCommentRepository {
    pool: MySqlPool,
}

impl MySqlCommentRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

pub const COMMENT_COLUMNS: &str = "id, topic_id, author_id, parent_id, body, created_at, \
    deleted_reason, deleted_by, deleted_at, deletion_penalty, edited_by, edited_at";

#[derive(FromRow)]
pub struct CommentRow {
    pub id: uuid::Uuid,
    pub topic_id: uuid::Uuid,
    pub author_id: uuid::Uuid,
    pub parent_id: Option<uuid::Uuid>,
    pub body: String,
    pub created_at: OffsetDateTime,
    pub deleted_reason: Option<String>,
    pub deleted_by: Option<uuid::Uuid>,
    pub deleted_at: Option<OffsetDateTime>,
    pub deletion_penalty: i32,
    pub edited_by: Option<uuid::Uuid>,
    pub edited_at: Option<OffsetDateTime>,
}

pub fn to_comment(row: CommentRow) -> Comment {
    let deleted = match (&row.deleted_reason, row.deleted_by, row.deleted_at) {
        (Some(reason), Some(moderator_id), Some(deleted_at)) => Some(Deletion::new(
            UserId::new(moderator_id),
            Reason::parse(reason).expect("stored reason is valid"),
            Penalty::parse(row.deletion_penalty).unwrap_or_default(),
            deleted_at,
        )),
        _ => None,
    };
    let edited = revision(row.edited_by, row.edited_at);
    Comment::from_parts(
        CommentId::new(row.id),
        TopicId::new(row.topic_id),
        UserId::new(row.author_id),
        row.parent_id.map(CommentId::new),
        Body::parse(&row.body).expect("stored body is valid"),
        row.created_at,
        deleted,
        edited,
    )
}

#[async_trait::async_trait]
impl CommentRepository for MySqlCommentRepository {
    async fn save(&self, comment: &Comment) {
        sqlx::query(
            "INSERT INTO comments (id, topic_id, author_id, parent_id, body, created_at, \
             deletion_penalty) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(comment.id().as_uuid())
        .bind(comment.topic_id().as_uuid())
        .bind(comment.author_id().as_uuid())
        .bind(comment.parent_id().map(|p| p.as_uuid()))
        .bind(comment.body().as_str())
        .bind(comment.created_at())
        .bind(stored_penalty(comment.deletion()))
        .execute(&self.pool)
        .await
        .expect("insert comment");
    }

    async fn update(&self, comment: &Comment) {
        sqlx::query(
            "UPDATE comments SET body = ?, deleted_reason = ?, deleted_by = ?, deleted_at = ?, \
             edited_by = ?, edited_at = ?, deletion_penalty = ? WHERE id = ?",
        )
        .bind(comment.body().as_str())
        .bind(comment.deletion().map(|d| d.reason().as_str()))
        .bind(comment.deletion().map(|d| d.moderator_id().as_uuid()))
        .bind(comment.deletion().map(|d| d.deleted_at()))
        .bind(comment.revision().map(|r| r.editor_id().as_uuid()))
        .bind(comment.revision().map(|r| r.edited_at()))
        .bind(stored_penalty(comment.deletion()))
        .bind(comment.id().as_uuid())
        .execute(&self.pool)
        .await
        .expect("update comment");
    }

    async fn find_by_id(&self, id: CommentId) -> Option<Comment> {
        sqlx::query_as::<_, CommentRow>(&format!(
            "SELECT {COMMENT_COLUMNS} FROM comments WHERE id = ?"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query comment")
        .map(to_comment)
    }

    async fn list_by_topic(&self, topic_id: TopicId, page: Page) -> Vec<Comment> {
        sqlx::query_as::<_, CommentRow>(&format!(
            "SELECT {COMMENT_COLUMNS} FROM comments WHERE topic_id = ? AND (\
             id IN (SELECT id FROM (\
             SELECT id FROM comments WHERE topic_id = ? AND parent_id IS NULL \
             ORDER BY created_at LIMIT ? OFFSET ?) r) \
             OR parent_id IN (SELECT id FROM (\
             SELECT id FROM comments WHERE topic_id = ? AND parent_id IS NULL \
             ORDER BY created_at LIMIT ? OFFSET ?) r2)) \
             ORDER BY created_at"
        ))
        .bind(topic_id.as_uuid())
        .bind(topic_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .bind(topic_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query comments")
        .into_iter()
        .map(to_comment)
        .collect()
    }

    async fn count_roots(&self, topic_id: TopicId) -> u64 {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM comments WHERE topic_id = ? AND parent_id IS NULL",
        )
        .bind(topic_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count root comments");
        count as u64
    }
}
