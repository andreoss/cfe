use crate::mysql::comment::{COMMENT_COLUMNS, CommentRow, to_comment};
use crate::mysql::topic::{TOPIC_COLUMNS, TopicRow, tags_for, to_topic};
use app::SearchRepository;
use domain::{ContentItem, Query};
use sqlx::{FromRow, MySqlPool};

pub struct MySqlSearchRepository {
    pool: MySqlPool,
}

impl MySqlSearchRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

const LIMIT: i64 = 50;

#[derive(FromRow)]
struct RankedTopic {
    #[sqlx(flatten)]
    topic: TopicRow,
    score: f32,
}

#[derive(FromRow)]
struct RankedComment {
    #[sqlx(flatten)]
    comment: CommentRow,
    score: f32,
}

#[async_trait::async_trait]
impl SearchRepository for MySqlSearchRepository {
    async fn search(&self, query: &Query) -> Vec<ContentItem> {
        let topic_rows = sqlx::query_as::<_, RankedTopic>(&format!(
            "SELECT {TOPIC_COLUMNS}, MATCH(title, body) AGAINST (?) AS score FROM topics \
             WHERE deleted_at IS NULL AND MATCH(title, body) AGAINST (?) \
             ORDER BY score DESC LIMIT ?"
        ))
        .bind(query.as_str())
        .bind(query.as_str())
        .bind(LIMIT)
        .fetch_all(&self.pool)
        .await
        .expect("query search topics");

        let comment_rows = sqlx::query_as::<_, RankedComment>(&format!(
            "SELECT {COMMENT_COLUMNS}, MATCH(body) AGAINST (?) AS score FROM comments \
             WHERE deleted_at IS NULL AND MATCH(body) AGAINST (?) \
             ORDER BY score DESC LIMIT ?"
        ))
        .bind(query.as_str())
        .bind(query.as_str())
        .bind(LIMIT)
        .fetch_all(&self.pool)
        .await
        .expect("query search comments");

        let mut ranked: Vec<(f32, ContentItem)> = Vec::new();
        for row in topic_rows {
            let tags = tags_for(&self.pool, row.topic.id).await;
            ranked.push((row.score, ContentItem::Topic(to_topic(row.topic, tags))));
        }
        for row in comment_rows {
            ranked.push((row.score, ContentItem::Comment(to_comment(row.comment))));
        }
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        ranked.truncate(LIMIT as usize);
        ranked.into_iter().map(|(_, item)| item).collect()
    }
}
