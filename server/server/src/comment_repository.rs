use app::CommentRepository;
use domain::{Body, Comment, CommentId, TopicId, UserId};
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

const SELECT_COLUMNS: &str = "id, topic_id, author_id, parent_id, body, created_at";

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    topic_id: uuid::Uuid,
    author_id: uuid::Uuid,
    parent_id: Option<uuid::Uuid>,
    body: String,
    created_at: OffsetDateTime,
}

fn to_comment(row: Row) -> Comment {
    Comment::new(
        CommentId::new(row.id),
        TopicId::new(row.topic_id),
        UserId::new(row.author_id),
        row.parent_id.map(CommentId::new),
        Body::parse(&row.body).expect("stored body is valid"),
        row.created_at,
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
