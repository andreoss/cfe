use crate::duckdb::comment::{COMMENT_COLUMNS, CommentRow, comment_row, to_comment};
use crate::duckdb::conn::Db;
use crate::duckdb::topic::{TOPIC_COLUMNS, TopicRow, tags_for, to_topic, topic_row};
use app::SearchRepository;
use domain::{ContentItem, Query};
use duckdb::types::Value;

pub struct DuckSearchRepository {
    db: Db,
}

impl DuckSearchRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const LIMIT: i64 = 50;

#[async_trait::async_trait]
impl SearchRepository for DuckSearchRepository {
    async fn search(&self, query: &Query) -> Vec<ContentItem> {
        let pattern = format!("%{}%", query.as_str().to_lowercase());
        let topic_params = vec![
            Value::Text(pattern.clone()),
            Value::Text(pattern.clone()),
            Value::BigInt(LIMIT),
        ];
        let topic_rows: Vec<TopicRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(&format!(
                        "SELECT {TOPIC_COLUMNS} FROM topics WHERE deleted_at IS NULL \
                         AND (lower(title) LIKE ? OR lower(body) LIKE ?) \
                         ORDER BY created_at DESC LIMIT ?"
                    ))
                    .expect("prepare search topics");
                let mapped = stmt
                    .query_map(duckdb::params_from_iter(topic_params.iter()), |row| {
                        Ok(topic_row(row))
                    })
                    .expect("query search topics");
                mapped.map(|r| r.expect("read topic")).collect()
            })
            .await;

        let comment_params = vec![Value::Text(pattern), Value::BigInt(LIMIT)];
        let comment_rows: Vec<CommentRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(&format!(
                        "SELECT {COMMENT_COLUMNS} FROM comments WHERE deleted_at IS NULL \
                         AND lower(body) LIKE ? ORDER BY created_at DESC LIMIT ?"
                    ))
                    .expect("prepare search comments");
                let mapped = stmt
                    .query_map(duckdb::params_from_iter(comment_params.iter()), |row| {
                        Ok(comment_row(row))
                    })
                    .expect("query search comments");
                mapped.map(|r| r.expect("read comment")).collect()
            })
            .await;

        let mut found = Vec::with_capacity(topic_rows.len() + comment_rows.len());
        for row in topic_rows {
            let tags = tags_for(&self.db, row.id).await;
            found.push(ContentItem::Topic(to_topic(row, tags)));
        }
        for row in comment_rows {
            found.push(ContentItem::Comment(to_comment(row)));
        }
        found
    }
}
