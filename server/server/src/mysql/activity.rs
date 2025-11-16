use crate::mysql::comment::{COMMENT_COLUMNS, CommentRow, to_comment};
use crate::mysql::topic::{TOPIC_COLUMNS, TopicRow, tags_for, to_topic};
use app::ActivityRepository;
use domain::ContentItem;
use sqlx::MySqlPool;
use time::OffsetDateTime;

pub struct MySqlActivityRepository {
    pool: MySqlPool,
}

impl MySqlActivityRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ActivityRepository for MySqlActivityRepository {
    async fn recent(&self, limit: u32) -> Vec<ContentItem> {
        let bound = limit as i64;
        let topic_rows = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {TOPIC_COLUMNS} FROM topics \
             WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ?"
        ))
        .bind(bound)
        .fetch_all(&self.pool)
        .await
        .expect("query recent topics");

        let comment_rows = sqlx::query_as::<_, CommentRow>(&format!(
            "SELECT {COMMENT_COLUMNS} FROM comments \
             WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ?"
        ))
        .bind(bound)
        .fetch_all(&self.pool)
        .await
        .expect("query recent comments");

        let mut dated: Vec<(OffsetDateTime, ContentItem)> = Vec::new();
        for row in topic_rows {
            let created_at = row.created_at;
            let tags = tags_for(&self.pool, row.id).await;
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
