use app::CommentRepository;
use domain::{Body, Comment, CommentId, Deletion, Reason, TopicId, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgCommentRepository {
    pool: PgPool,
}

impl PgCommentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, topic_id, author_id, parent_id, body, created_at, \
    deleted_reason, deleted_by, deleted_at";

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    topic_id: uuid::Uuid,
    author_id: uuid::Uuid,
    parent_id: Option<uuid::Uuid>,
    body: String,
    created_at: OffsetDateTime,
    deleted_reason: Option<String>,
    deleted_by: Option<uuid::Uuid>,
    deleted_at: Option<OffsetDateTime>,
}

fn to_deletion(row: &Row) -> Option<Deletion> {
    match (&row.deleted_reason, row.deleted_by, row.deleted_at) {
        (Some(reason), Some(moderator_id), Some(deleted_at)) => Some(Deletion::new(
            UserId::new(moderator_id),
            Reason::parse(reason).expect("stored reason is valid"),
            deleted_at,
        )),
        _ => None,
    }
}

fn to_comment(row: Row) -> Comment {
    let deleted = to_deletion(&row);
    Comment::from_parts(
        CommentId::new(row.id),
        TopicId::new(row.topic_id),
        UserId::new(row.author_id),
        row.parent_id.map(CommentId::new),
        Body::parse(&row.body).expect("stored body is valid"),
        row.created_at,
        deleted,
    )
}

#[async_trait::async_trait]
impl CommentRepository for PgCommentRepository {
    async fn save(&self, comment: &Comment) {
        sqlx::query(
            "INSERT INTO comments (id, topic_id, author_id, parent_id, body, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(comment.id().as_uuid())
        .bind(comment.topic_id().as_uuid())
        .bind(comment.author_id().as_uuid())
        .bind(comment.parent_id().map(|p| p.as_uuid()))
        .bind(comment.body().as_str())
        .bind(comment.created_at())
        .execute(&self.pool)
        .await
        .expect("insert comment");
    }

    async fn update(&self, comment: &Comment) {
        sqlx::query(
            "UPDATE comments SET body = $2, deleted_reason = $3, deleted_by = $4, \
             deleted_at = $5 WHERE id = $1",
        )
        .bind(comment.id().as_uuid())
        .bind(comment.body().as_str())
        .bind(comment.deletion().map(|d| d.reason().as_str()))
        .bind(comment.deletion().map(|d| d.moderator_id().as_uuid()))
        .bind(comment.deletion().map(|d| d.deleted_at()))
        .execute(&self.pool)
        .await
        .expect("update comment");
    }

    async fn find_by_id(&self, id: CommentId) -> Option<Comment> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM comments WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_id")
        .map(to_comment)
    }

    async fn list_by_topic(&self, topic_id: TopicId) -> Vec<Comment> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM comments WHERE topic_id = $1 ORDER BY created_at ASC"
        ))
        .bind(topic_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query list_by_topic")
        .into_iter()
        .map(to_comment)
        .collect()
    }
}
