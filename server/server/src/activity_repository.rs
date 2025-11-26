use crate::topic_repository::{SELECT_COLUMNS, TopicRow, to_topic};
use app::ActivityRepository;
use domain::{Body, Comment, CommentId, ContentItem, Revision, TopicId, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgActivityRepository {
    pool: PgPool,
}

impl PgActivityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct CommentRow {
    id: uuid::Uuid,
    topic_id: uuid::Uuid,
    author_id: uuid::Uuid,
    parent_id: Option<uuid::Uuid>,
    body: String,
    created_at: OffsetDateTime,
    edited_by: Option<uuid::Uuid>,
    edited_at: Option<OffsetDateTime>,
}

fn revision(by: Option<uuid::Uuid>, at: Option<OffsetDateTime>) -> Option<Revision> {
    match (by, at) {
        (Some(editor_id), Some(edited_at)) => {
            Some(Revision::new(UserId::new(editor_id), edited_at))
        }
        _ => None,
    }
}

#[async_trait::async_trait]
impl ActivityRepository for PgActivityRepository {
    async fn recent(&self, limit: u32) -> Vec<ContentItem> {
        let bound = limit as i64;
        let topic_rows = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM topics WHERE deleted_at IS NULL \
             ORDER BY created_at DESC LIMIT $1"
        ))
        .bind(bound)
        .fetch_all(&self.pool)
        .await
        .expect("query recent topics");

        let comment_rows = sqlx::query_as::<_, CommentRow>(
            "SELECT id, topic_id, author_id, parent_id, body, created_at, edited_by, edited_at \
             FROM comments WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT $1",
        )
        .bind(bound)
        .fetch_all(&self.pool)
        .await
        .expect("query recent comments");

        let mut dated: Vec<(OffsetDateTime, ContentItem)> = Vec::new();
        for row in topic_rows {
            let topic = to_topic(row);
            dated.push((topic.created_at(), ContentItem::Topic(topic)));
        }
        for row in comment_rows {
            let created_at = row.created_at;
            dated.push((
                created_at,
                ContentItem::Comment(Comment::from_parts(
                    CommentId::new(row.id),
                    TopicId::new(row.topic_id),
                    UserId::new(row.author_id),
                    row.parent_id.map(CommentId::new),
                    Body::parse(&row.body).expect("stored body is valid"),
                    created_at,
                    None,
                    revision(row.edited_by, row.edited_at),
                )),
            ));
        }
        dated.sort_by(|a, b| b.0.cmp(&a.0));
        dated.truncate(limit as usize);
        dated.into_iter().map(|(_, item)| item).collect()
    }
}
