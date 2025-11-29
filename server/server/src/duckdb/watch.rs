use crate::duckdb::conn::Db;
use crate::duckdb::topic::{
    count, limit_value, load_topics, offset_value, read_time, read_uuid, tagged_columns,
    time_to_value, uuid_value,
};
use app::WatchRepository;
use domain::{Page, Topic, TopicId, UserId, Watch};
use duckdb::types::Value;

pub struct DuckWatchRepository {
    db: Db,
}

impl DuckWatchRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl WatchRepository for DuckWatchRepository {
    async fn save(&self, watch: &Watch) {
        let params = vec![
            uuid_value(watch.user_id().as_uuid()),
            uuid_value(watch.topic_id().as_uuid()),
            time_to_value(watch.created_at()),
        ];
        self.db
            .execute(
                "INSERT INTO watches (user_id, topic_id, created_at) VALUES (?, ?, ?) \
                 ON CONFLICT (user_id, topic_id) DO NOTHING",
                params,
            )
            .await;
    }

    async fn delete(&self, user_id: UserId, topic_id: TopicId) {
        self.db
            .execute(
                "DELETE FROM watches WHERE user_id = ? AND topic_id = ?",
                vec![
                    uuid_value(user_id.as_uuid()),
                    uuid_value(topic_id.as_uuid()),
                ],
            )
            .await;
    }

    async fn find(&self, user_id: UserId, topic_id: TopicId) -> Option<Watch> {
        let params = vec![
            uuid_value(user_id.as_uuid()),
            uuid_value(topic_id.as_uuid()),
        ];
        let found: Vec<Watch> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT user_id, topic_id, created_at FROM watches \
                         WHERE user_id = ? AND topic_id = ?",
                    )
                    .expect("prepare watch");
                let mapped = stmt
                    .query_map(duckdb::params_from_iter(params.iter()), |row| {
                        Ok(Watch::new(
                            UserId::new(read_uuid(row, 0)),
                            TopicId::new(read_uuid(row, 1)),
                            read_time(row, 2),
                        ))
                    })
                    .expect("query watch");
                mapped.map(|r| r.expect("read watch")).collect()
            })
            .await;
        found.into_iter().next()
    }

    async fn watchers(&self, topic_id: TopicId) -> Vec<UserId> {
        let params = vec![uuid_value(topic_id.as_uuid())];
        self.db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT user_id FROM watches WHERE topic_id = ? ORDER BY created_at")
                    .expect("prepare watchers");
                let mapped = stmt
                    .query_map(duckdb::params_from_iter(params.iter()), |row| {
                        Ok(UserId::new(read_uuid(row, 0)))
                    })
                    .expect("query watchers");
                mapped.map(|r| r.expect("read watcher")).collect()
            })
            .await
    }

    async fn list_topics(&self, user_id: UserId, page: Page) -> Vec<Topic> {
        let columns = tagged_columns();
        let sql = format!(
            "SELECT {columns} FROM watches w JOIN topics t ON t.id = w.topic_id \
             WHERE w.user_id = ? AND t.deleted_at IS NULL \
             ORDER BY w.created_at DESC LIMIT ? OFFSET ?"
        );
        let params: Vec<Value> = vec![
            uuid_value(user_id.as_uuid()),
            limit_value(page),
            offset_value(page),
        ];
        load_topics(&self.db, sql, params).await
    }

    async fn count_topics(&self, user_id: UserId) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM watches w JOIN topics t ON t.id = w.topic_id \
             WHERE w.user_id = ? AND t.deleted_at IS NULL",
            vec![uuid_value(user_id.as_uuid())],
        )
        .await
    }
}
