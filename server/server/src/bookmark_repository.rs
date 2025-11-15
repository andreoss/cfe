use app::BookmarkRepository;
use domain::{
    Body, Bookmark, Deletion, Reason, Revision, SectionId, TagSet, Title, Topic, TopicId, UserId,
};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgBookmarkRepository {
    pool: PgPool,
}

impl PgBookmarkRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct TopicRow {
    id: uuid::Uuid,
    section_id: uuid::Uuid,
    author_id: uuid::Uuid,
    title: String,
    body: String,
    tags: Vec<String>,
    created_at: OffsetDateTime,
    deleted_reason: Option<String>,
    deleted_by: Option<uuid::Uuid>,
    deleted_at: Option<OffsetDateTime>,
    edited_by: Option<uuid::Uuid>,
    edited_at: Option<OffsetDateTime>,
}

fn to_topic(row: TopicRow) -> Topic {
    let deleted = match (&row.deleted_reason, row.deleted_by, row.deleted_at) {
        (Some(reason), Some(moderator_id), Some(deleted_at)) => Some(Deletion::new(
            UserId::new(moderator_id),
            Reason::parse(reason).expect("stored reason is valid"),
            deleted_at,
        )),
        _ => None,
    };
    let edited = match (row.edited_by, row.edited_at) {
        (Some(editor_id), Some(edited_at)) => {
            Some(Revision::new(UserId::new(editor_id), edited_at))
        }
        _ => None,
    };
    Topic::from_parts(
        TopicId::new(row.id),
        SectionId::new(row.section_id),
        UserId::new(row.author_id),
        Title::parse(&row.title).expect("stored title is valid"),
        Body::parse(&row.body).expect("stored body is valid"),
        TagSet::parse(&row.tags).expect("stored tags are valid"),
        row.created_at,
        deleted,
        edited,
    )
}

#[async_trait::async_trait]
impl BookmarkRepository for PgBookmarkRepository {
    async fn save(&self, bookmark: &Bookmark) {
        sqlx::query(
            "INSERT INTO bookmarks (user_id, topic_id, created_at) VALUES ($1, $2, $3) \
             ON CONFLICT (user_id, topic_id) DO NOTHING",
        )
        .bind(bookmark.user_id().as_uuid())
        .bind(bookmark.topic_id().as_uuid())
        .bind(bookmark.created_at())
        .execute(&self.pool)
        .await
        .expect("insert bookmark");
    }

    async fn delete(&self, user_id: UserId, topic_id: TopicId) {
        sqlx::query("DELETE FROM bookmarks WHERE user_id = $1 AND topic_id = $2")
            .bind(user_id.as_uuid())
            .bind(topic_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete bookmark");
    }

    async fn exists(&self, user_id: UserId, topic_id: TopicId) -> bool {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM bookmarks WHERE user_id = $1 AND topic_id = $2",
        )
        .bind(user_id.as_uuid())
        .bind(topic_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count bookmark");
        count > 0
    }

    async fn list_topics(&self, user_id: UserId) -> Vec<Topic> {
        sqlx::query_as::<_, TopicRow>(
            "SELECT t.id, t.section_id, t.author_id, t.title, t.body, t.tags, t.created_at, \
             t.deleted_reason, t.deleted_by, t.deleted_at, t.edited_by, t.edited_at \
             FROM bookmarks b JOIN topics t ON t.id = b.topic_id \
             WHERE b.user_id = $1 AND t.deleted_at IS NULL \
             ORDER BY b.created_at DESC",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query bookmarked topics")
        .into_iter()
        .map(to_topic)
        .collect()
    }
}
