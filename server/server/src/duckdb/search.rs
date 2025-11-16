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

pub fn like_pattern(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.to_lowercase().chars() {
        if matches!(ch, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    format!("%{escaped}%")
}

#[async_trait::async_trait]
impl SearchRepository for DuckSearchRepository {
    async fn search(&self, query: &Query) -> Vec<ContentItem> {
        let pattern = like_pattern(query.as_str());
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
                         AND (lower(title) LIKE ? ESCAPE '\\' \
                         OR lower(body) LIKE ? ESCAPE '\\') \
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
                         AND lower(body) LIKE ? ESCAPE '\\' ORDER BY created_at DESC LIMIT ?"
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
        found.truncate(LIMIT as usize);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_a_plain_term_in_wildcards_and_lowercases_it() {
        assert_eq!(like_pattern("Adapters"), "%adapters%");
    }

    #[test]
    fn escapes_wildcards_so_a_user_cannot_match_everything() {
        assert_eq!(like_pattern("%"), "%\\%%");
        assert_eq!(like_pattern("_"), "%\\_%");
        assert_eq!(like_pattern("a%b_c"), "%a\\%b\\_c%");
    }

    #[test]
    fn escapes_the_escape_character_itself() {
        assert_eq!(like_pattern("a\\b"), "%a\\\\b%");
    }
}
