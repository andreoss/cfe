use crate::duckdb::comment::{COMMENT_COLUMNS, CommentRow, comment_row, to_comment};
use crate::duckdb::conn::Db;
use crate::duckdb::topic::{TOPIC_COLUMNS, TopicRow, tags_for, to_topic, topic_row};
use app::ActivityRepository;
use domain::ContentItem;
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckActivityRepository {
    db: Db,
}

impl DuckActivityRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl ActivityRepository for DuckActivityRepository {
    async fn recent(&self, limit: u32) -> Vec<ContentItem> {
        let bound = limit as i64;
        let topic_sql = format!(
            "SELECT {TOPIC_COLUMNS} FROM topics \
             WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ?"
        );
        let topic_rows: Vec<TopicRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn.prepare(&topic_sql).expect("prepare recent topics");
                let mapped = stmt
                    .query_map([Value::BigInt(bound)], |row| Ok(topic_row(row)))
                    .expect("query recent topics");
                mapped.map(|r| r.expect("read topic")).collect()
            })
            .await;

        let comment_sql = format!(
            "SELECT {COMMENT_COLUMNS} FROM comments \
             WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ?"
        );
        let comment_rows: Vec<CommentRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn.prepare(&comment_sql).expect("prepare recent comments");
                let mapped = stmt
                    .query_map([Value::BigInt(bound)], |row| Ok(comment_row(row)))
                    .expect("query recent comments");
                mapped.map(|r| r.expect("read comment")).collect()
            })
            .await;

        let mut dated: Vec<(OffsetDateTime, ContentItem)> = Vec::new();
        for row in topic_rows {
            let created_at = row.created_at;
            let tags = tags_for(&self.db, row.id).await;
            dated.push((created_at, ContentItem::Topic(to_topic(row, tags))));
        }
        for row in comment_rows {
            let created_at = row.created_at;
            dated.push((created_at, ContentItem::Comment(to_comment(row))));
        }
        dated.sort_by(|a, b| b.0.cmp(&a.0));
        dated.truncate(limit as usize);
        dated.into_iter().map(|(_, item)| item).collect()
    }
}
