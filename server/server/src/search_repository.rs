use app::SearchRepository;
use domain::{
    Body, Comment, CommentId, Query, Revision, SearchHit, SectionId, TagSet, Title, Topic, TopicId,
    UserId,
};
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
struct TopicRow {
    id: uuid::Uuid,
    section_id: uuid::Uuid,
    author_id: uuid::Uuid,
    title: String,
    body: String,
    tags: Vec<String>,
    created_at: OffsetDateTime,
    edited_by: Option<uuid::Uuid>,
    edited_at: Option<OffsetDateTime>,
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

#[async_trait::async_trait]
impl SearchRepository for PgSearchRepository {
    async fn search(&self, query: &Query) -> Vec<SearchHit> {
        let topic_rows = sqlx::query_as::<_, TopicRow>(
            "SELECT id, section_id, author_id, title, body, tags, created_at, edited_by, \
             edited_at, ts_rank(to_tsvector('english', title || ' ' || body), \
             plainto_tsquery('english', $1)) AS rank \
             FROM topics \
             WHERE deleted_at IS NULL \
             AND to_tsvector('english', title || ' ' || body) \
             @@ plainto_tsquery('english', $1) \
             ORDER BY rank DESC LIMIT $2",
        )
        .bind(query.as_str())
        .bind(LIMIT)
        .fetch_all(&self.pool)
        .await
        .expect("query search topics");

        let comment_rows = sqlx::query_as::<_, CommentRow>(
            "SELECT id, topic_id, author_id, parent_id, body, created_at, edited_by, edited_at, \
             ts_rank(to_tsvector('english', body), plainto_tsquery('english', $1)) AS rank \
             FROM comments \
             WHERE deleted_at IS NULL \
             AND to_tsvector('english', body) @@ plainto_tsquery('english', $1) \
             ORDER BY rank DESC LIMIT $2",
        )
        .bind(query.as_str())
        .bind(LIMIT)
        .fetch_all(&self.pool)
        .await
        .expect("query search comments");

        let mut ranked: Vec<(f32, SearchHit)> = Vec::new();
        for row in topic_rows {
            let hit = SearchHit::Topic(Topic::from_parts(
                TopicId::new(row.id),
                SectionId::new(row.section_id),
                UserId::new(row.author_id),
                Title::parse(&row.title).expect("stored title is valid"),
                Body::parse(&row.body).expect("stored body is valid"),
                TagSet::parse(&row.tags).expect("stored tags are valid"),
                row.created_at,
                None,
                revision(row.edited_by, row.edited_at),
            ));
            ranked.push((row.rank, hit));
        }
        for row in comment_rows {
            let hit = SearchHit::Comment(Comment::from_parts(
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
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        ranked.truncate(LIMIT as usize);
        ranked.into_iter().map(|(_, hit)| hit).collect()
    }
}
