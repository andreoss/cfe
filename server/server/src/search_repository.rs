use crate::topic_repository::{SELECT_COLUMNS, TopicRow, to_topic};
use app::SearchRepository;
use domain::{Body, Comment, CommentId, ContentItem, Criteria, Order, Revision, TopicId, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgSearchRepository {
    pool: PgPool,
}

impl PgSearchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const LIMIT: i64 = 50;

#[derive(FromRow)]
struct RankedTopic {
    #[sqlx(flatten)]
    topic: TopicRow,
    rank: f32,
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
    rank: f32,
}

fn revision(by: Option<uuid::Uuid>, at: Option<OffsetDateTime>) -> Option<Revision> {
    match (by, at) {
        (Some(editor_id), Some(edited_at)) => {
            Some(Revision::new(UserId::new(editor_id), edited_at))
        }
        _ => None,
    }
}

fn ordering(order: Order) -> &'static str {
    match order {
        Order::Relevance => "rank DESC",
        Order::Newest => "created_at DESC",
        Order::Oldest => "created_at ASC",
    }
}

fn arrange(order: Order, mut ranked: Vec<(f32, ContentItem)>) -> Vec<ContentItem> {
    match order {
        Order::Relevance => ranked.sort_by(|a, b| b.0.total_cmp(&a.0)),
        Order::Newest => ranked.sort_by(|a, b| b.1.written_at().cmp(&a.1.written_at())),
        Order::Oldest => ranked.sort_by(|a, b| a.1.written_at().cmp(&b.1.written_at())),
    }
    ranked.truncate(LIMIT as usize);
    ranked.into_iter().map(|(_, hit)| hit).collect()
}

#[async_trait::async_trait]
impl SearchRepository for PgSearchRepository {
    async fn search(&self, criteria: &Criteria) -> Vec<ContentItem> {
        let query = criteria.query();
        let by = ordering(criteria.order());
        let topic_rows = if criteria.scope().covers_topics() {
            sqlx::query_as::<_, RankedTopic>(&format!(
                "SELECT {SELECT_COLUMNS}, ts_rank(to_tsvector('english', title || ' ' || body), \
                 plainto_tsquery('english', $1)) AS rank \
                 FROM topics \
                 WHERE deleted_at IS NULL \
                 AND to_tsvector('english', title || ' ' || body) \
                 @@ plainto_tsquery('english', $1) \
                 ORDER BY {by} LIMIT $2"
            ))
            .bind(query.as_str())
            .bind(LIMIT)
            .fetch_all(&self.pool)
            .await
            .expect("query search topics")
        } else {
            Vec::new()
        };

        let comment_rows = if criteria.scope().covers_comments() {
            sqlx::query_as::<_, CommentRow>(&format!(
                "SELECT id, topic_id, author_id, parent_id, body, created_at, edited_by, \
                 edited_at, \
                 ts_rank(to_tsvector('english', body), plainto_tsquery('english', $1)) AS rank \
                 FROM comments \
                 WHERE deleted_at IS NULL \
                 AND to_tsvector('english', body) @@ plainto_tsquery('english', $1) \
                 ORDER BY {by} LIMIT $2"
            ))
            .bind(query.as_str())
            .bind(LIMIT)
            .fetch_all(&self.pool)
            .await
            .expect("query search comments")
        } else {
            Vec::new()
        };

        let mut ranked: Vec<(f32, ContentItem)> = Vec::new();
        for row in topic_rows {
            ranked.push((row.rank, ContentItem::Topic(to_topic(row.topic))));
        }
        for row in comment_rows {
            let hit = ContentItem::Comment(Comment::from_parts(
                CommentId::new(row.id),
                TopicId::new(row.topic_id),
                UserId::new(row.author_id),
                row.parent_id.map(CommentId::new),
                Body::parse(&row.body).expect("stored body is valid"),
                row.created_at,
                None,
                revision(row.edited_by, row.edited_at),
            ));
            ranked.push((row.rank, hit));
        }
        arrange(criteria.order(), ranked)
    }
}
