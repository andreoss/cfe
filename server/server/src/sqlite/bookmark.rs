use crate::sqlite::conn::Db;
use crate::sqlite::topic::{
    count, limit_value, load_topics, offset_value, tagged_columns, time_to_value, uuid_value,
};
use app::BookmarkRepository;
use domain::{Bookmark, Page, Topic, TopicId, UserId};
use rusqlite::types::Value;

pub struct SqliteBookmarkRepository {
    db: Db,
}

impl SqliteBookmarkRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl BookmarkRepository for SqliteBookmarkRepository {
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
                    rusqlite::params_from_iter(params.iter()),
                    |row| row.get(0),
                )
                .expect("count bookmark")
            })
            .await;
        count > 0
    }

    async fn list_topics(&self, user_id: UserId, page: Page) -> Vec<Topic> {
        let columns = tagged_columns();
        let sql = format!(
            "SELECT {columns} FROM bookmarks b JOIN topics t ON t.id = b.topic_id \
             WHERE b.user_id = ? AND t.deleted_at IS NULL \
             ORDER BY b.created_at DESC LIMIT ? OFFSET ?"
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
            "SELECT COUNT(*) FROM bookmarks b JOIN topics t ON t.id = b.topic_id \
             WHERE b.user_id = ? AND t.deleted_at IS NULL",
            vec![uuid_value(user_id.as_uuid())],
        )
        .await
    }
}
