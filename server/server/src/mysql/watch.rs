use crate::mysql::topic::{TOPIC_COLUMNS, TopicRow, tags_for, to_topic};
use app::WatchRepository;
use domain::{Page, Topic, TopicId, UserId, Watch};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlWatchRepository {
    pool: MySqlPool,
}

impl MySqlWatchRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct WatchRow {
    user_id: uuid::Uuid,
    topic_id: uuid::Uuid,
    created_at: OffsetDateTime,
}

fn to_watch(row: WatchRow) -> Watch {
    Watch::new(
        UserId::new(row.user_id),
        TopicId::new(row.topic_id),
        row.created_at,
    )
}

#[async_trait::async_trait]
impl WatchRepository for MySqlWatchRepository {
    async fn save(&self, watch: &Watch) {
        sqlx::query("INSERT IGNORE INTO watches (user_id, topic_id, created_at) VALUES (?, ?, ?)")
            .bind(watch.user_id().as_uuid())
            .bind(watch.topic_id().as_uuid())
            .bind(watch.created_at())
            .execute(&self.pool)
            .await
            .expect("insert watch");
    }

    async fn delete(&self, user_id: UserId, topic_id: TopicId) {
        sqlx::query("DELETE FROM watches WHERE user_id = ? AND topic_id = ?")
            .bind(user_id.as_uuid())
            .bind(topic_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete watch");
    }

    async fn find(&self, user_id: UserId, topic_id: TopicId) -> Option<Watch> {
        sqlx::query_as::<_, WatchRow>(
            "SELECT user_id, topic_id, created_at FROM watches \
             WHERE user_id = ? AND topic_id = ?",
        )
        .bind(user_id.as_uuid())
        .bind(topic_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query watch")
        .map(to_watch)
    }

    async fn watchers(&self, topic_id: TopicId) -> Vec<UserId> {
        sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT user_id FROM watches WHERE topic_id = ? ORDER BY created_at",
        )
        .bind(topic_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query watchers")
        .into_iter()
        .map(UserId::new)
        .collect()
    }

    async fn list_topics(&self, user_id: UserId, page: Page) -> Vec<Topic> {
        let rows = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {} FROM watches w JOIN topics t ON t.id = w.topic_id \
             WHERE w.user_id = ? AND t.deleted_at IS NULL \
             ORDER BY w.created_at DESC LIMIT ? OFFSET ?",
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
        .expect("query watched topics");
        let mut topics = Vec::with_capacity(rows.len());
        for row in rows {
            let tags = tags_for(&self.pool, row.id).await;
            topics.push(to_topic(row, tags));
        }
        topics
    }

    async fn count_topics(&self, user_id: UserId) -> u64 {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM watches w JOIN topics t ON t.id = w.topic_id \
             WHERE w.user_id = ? AND t.deleted_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count watched topics");
        count as u64
    }
}
