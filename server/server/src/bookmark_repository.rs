use crate::topic_repository::{TopicRow, aliased_columns, to_topic};
use app::BookmarkRepository;
use domain::{Bookmark, Page, Topic, TopicId, UserId};
use sqlx::PgPool;

pub struct PgBookmarkRepository {
    pool: PgPool,
}

impl PgBookmarkRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
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

    async fn list_topics(&self, user_id: UserId, page: Page) -> Vec<Topic> {
        sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {} FROM bookmarks b JOIN topics t ON t.id = b.topic_id \
             WHERE b.user_id = $1 AND t.deleted_at IS NULL \
             ORDER BY b.created_at DESC \
             LIMIT $2 OFFSET $3",
            aliased_columns("t")
        ))
        .bind(user_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query bookmarked topics")
        .into_iter()
        .map(to_topic)
        .collect()
    }

    async fn count_topics(&self, user_id: UserId) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM bookmarks b JOIN topics t ON t.id = b.topic_id \
             WHERE b.user_id = $1 AND t.deleted_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count bookmarked topics");
        count as u64
    }
}
