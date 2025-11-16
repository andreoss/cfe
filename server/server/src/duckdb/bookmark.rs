use crate::duckdb::conn::Db;
use crate::duckdb::topic::{
    TOPIC_COLUMNS, TopicRow, tags_for, time_to_value, to_topic, topic_row, uuid_value,
};
use app::BookmarkRepository;
use domain::{Bookmark, Topic, TopicId, UserId};
use duckdb::types::Value;

pub struct DuckBookmarkRepository {
    db: Db,
}

impl DuckBookmarkRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl BookmarkRepository for DuckBookmarkRepository {
    async fn save(&self, bookmark: &Bookmark) {
        let params = vec![
            uuid_value(bookmark.user_id().as_uuid()),
            uuid_value(bookmark.topic_id().as_uuid()),
            time_to_value(bookmark.created_at()),
        ];
        self.db
            .execute(
                "INSERT INTO bookmarks (user_id, topic_id, created_at) VALUES (?, ?, ?) \
                 ON CONFLICT (user_id, topic_id) DO NOTHING",
                params,
            )
            .await;
    }

    async fn delete(&self, user_id: UserId, topic_id: TopicId) {
        self.db
            .execute(
                "DELETE FROM bookmarks WHERE user_id = ? AND topic_id = ?",
                vec![
                    uuid_value(user_id.as_uuid()),
                    uuid_value(topic_id.as_uuid()),
                ],
            )
            .await;
    }

    async fn exists(&self, user_id: UserId, topic_id: TopicId) -> bool {
        let params = vec![
            uuid_value(user_id.as_uuid()),
            uuid_value(topic_id.as_uuid()),
        ];
        let count: i64 = self
            .db
            .call(move |conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM bookmarks WHERE user_id = ? AND topic_id = ?",
                    duckdb::params_from_iter(params.iter()),
                    |row| row.get(0),
                )
                .expect("count bookmark")
            })
            .await;
        count > 0
    }

    async fn list_topics(&self, user_id: UserId) -> Vec<Topic> {
        let columns = TOPIC_COLUMNS
            .split(", ")
            .map(|c| format!("t.{c}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT {columns} FROM bookmarks b JOIN topics t ON t.id = b.topic_id \
             WHERE b.user_id = ? AND t.deleted_at IS NULL ORDER BY b.created_at DESC"
        );
        let params: Vec<Value> = vec![uuid_value(user_id.as_uuid())];
        let rows: Vec<TopicRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn.prepare(&sql).expect("prepare bookmarked topics");
                let mapped = stmt
                    .query_map(duckdb::params_from_iter(params.iter()), |row| {
                        Ok(topic_row(row))
                    })
                    .expect("query bookmarked topics");
                mapped.map(|r| r.expect("read topic")).collect()
            })
            .await;
        let mut topics = Vec::with_capacity(rows.len());
        for row in rows {
            let tags = tags_for(&self.db, row.id).await;
            topics.push(to_topic(row, tags));
        }
        topics
    }
}
