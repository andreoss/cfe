use crate::topic_repository::{TopicRow, aliased_columns, to_topic};
use app::WatchRepository;
use domain::{Page, Topic, TopicId, UserId, Watch};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgWatchRepository {
    pool: PgPool,
}

impl PgWatchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct Row {
    user_id: uuid::Uuid,
    topic_id: uuid::Uuid,
    created_at: OffsetDateTime,
}

fn to_watch(row: Row) -> Watch {
    Watch::new(
        UserId::new(row.user_id),
        TopicId::new(row.topic_id),
        row.created_at,
    )
}

#[async_trait::async_trait]
impl WatchRepository for PgWatchRepository {
    async fn save(&self, watch: &Watch) {
        sqlx::query(
            "INSERT INTO watches (user_id, topic_id, created_at) VALUES ($1, $2, $3) \
             ON CONFLICT (user_id, topic_id) DO NOTHING",
        )
        .bind(watch.user_id().as_uuid())
        .bind(watch.topic_id().as_uuid())
        .bind(watch.created_at())
        .execute(&self.pool)
        .await
        .expect("insert watch");
    }

    async fn delete(&self, user_id: UserId, topic_id: TopicId) {
        sqlx::query("DELETE FROM watches WHERE user_id = $1 AND topic_id = $2")
            .bind(user_id.as_uuid())
            .bind(topic_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete watch");
    }

    async fn find(&self, user_id: UserId, topic_id: TopicId) -> Option<Watch> {
        sqlx::query_as::<_, Row>(
            "SELECT user_id, topic_id, created_at FROM watches \
             WHERE user_id = $1 AND topic_id = $2",
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
            "SELECT user_id FROM watches WHERE topic_id = $1 ORDER BY created_at",
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
        sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {} FROM watches w JOIN topics t ON t.id = w.topic_id \
             WHERE w.user_id = $1 AND t.deleted_at IS NULL \
             ORDER BY w.created_at DESC \
             LIMIT $2 OFFSET $3",
            aliased_columns("t")
        ))
        .bind(user_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query watched topics")
        .into_iter()
        .map(to_topic)
        .collect()
    }

    async fn count_topics(&self, user_id: UserId) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM watches w JOIN topics t ON t.id = w.topic_id \
             WHERE w.user_id = $1 AND t.deleted_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count watched topics");
        count as u64
    }
}
