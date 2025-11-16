use crate::mysql::topic::{TOPIC_COLUMNS, TopicRow, tags_for, to_topic};
use app::BookmarkRepository;
use domain::{Bookmark, Page, Topic, TopicId, UserId};
use sqlx::MySqlPool;

pub struct MySqlBookmarkRepository {
    pool: MySqlPool,
}

impl MySqlBookmarkRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BookmarkRepository for MySqlBookmarkRepository {
    async fn save(&self, bookmark: &Bookmark) {
        sqlx::query(
            "INSERT IGNORE INTO bookmarks (user_id, topic_id, created_at) VALUES (?, ?, ?)",
        )
        .bind(bookmark.user_id().as_uuid())
        .bind(bookmark.topic_id().as_uuid())
        .bind(bookmark.created_at())
        .execute(&self.pool)
        .await
        .expect("insert bookmark");
    }

    async fn delete(&self, user_id: UserId, topic_id: TopicId) {
        sqlx::query("DELETE FROM bookmarks WHERE user_id = ? AND topic_id = ?")
            .bind(user_id.as_uuid())
            .bind(topic_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete bookmark");
    }

    async fn exists(&self, user_id: UserId, topic_id: TopicId) -> bool {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM bookmarks WHERE user_id = ? AND topic_id = ?",
        )
        .bind(user_id.as_uuid())
        .bind(topic_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count bookmark");
        count > 0
    }

    async fn list_topics(&self, user_id: UserId, page: Page) -> Vec<Topic> {
        let rows = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {} FROM bookmarks b JOIN topics t ON t.id = b.topic_id \
             WHERE b.user_id = ? AND t.deleted_at IS NULL \
             ORDER BY b.created_at DESC LIMIT ? OFFSET ?",
            TOPIC_COLUMNS
                .split(", ")
                .map(|c| format!("t.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .bind(user_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query bookmarked topics");
        let mut topics = Vec::with_capacity(rows.len());
        for row in rows {
            let tags = tags_for(&self.pool, row.id).await;
            topics.push(to_topic(row, tags));
        }
        topics
    }

    async fn count_topics(&self, user_id: UserId) -> u64 {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM bookmarks b JOIN topics t ON t.id = b.topic_id \
             WHERE b.user_id = ? AND t.deleted_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count bookmarked topics");
        count as u64
    }
}
