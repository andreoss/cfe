use crate::sqlite::comment::{COMMENT_COLUMNS, CommentRow, comment_row, to_comment};
use crate::sqlite::conn::Db;
use crate::sqlite::topic::{TOPIC_COLUMNS, TopicRow, tags_for, to_topic, topic_row};
use app::SearchRepository;
use domain::{ContentItem, Criteria, Order};
use rusqlite::types::Value;

pub struct SqliteSearchRepository {
    db: Db,
}

impl SqliteSearchRepository {
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

pub fn ordering(order: Order, rank: &str) -> String {
    match order {
        Order::Relevance => format!("{rank}, created_at DESC"),
        Order::Newest => "created_at DESC".to_owned(),
        Order::Oldest => "created_at ASC".to_owned(),
    }
}

fn arrange(order: Order, mut ranked: Vec<(i32, ContentItem)>) -> Vec<ContentItem> {
    match order {
        Order::Relevance => {
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.written_at().cmp(&a.1.written_at())))
        }
        Order::Newest => ranked.sort_by(|a, b| b.1.written_at().cmp(&a.1.written_at())),
        Order::Oldest => ranked.sort_by(|a, b| a.1.written_at().cmp(&b.1.written_at())),
    }
    ranked.truncate(LIMIT as usize);
    ranked.into_iter().map(|(_, item)| item).collect()
}

const TOPIC_RANK: &str = "CASE WHEN lower(title) LIKE ? ESCAPE '\\' THEN 2 ELSE 1 END";

#[async_trait::async_trait]
impl SearchRepository for SqliteSearchRepository {
    async fn search(&self, criteria: &Criteria) -> Vec<ContentItem> {
        let pattern = like_pattern(criteria.query().as_str());
        let order = criteria.order();
        let topic_rows: Vec<(i32, TopicRow)> = if criteria.scope().covers_topics() {
            let by = ordering(order, "rank DESC");
            let params = vec![
                Value::Text(pattern.clone()),
                Value::Text(pattern.clone()),
                Value::Text(pattern.clone()),
                Value::Integer(LIMIT),
            ];
            self.db
                .call(move |conn| {
                    let mut stmt = conn
                        .prepare(&format!(
                            "SELECT {TOPIC_COLUMNS}, {TOPIC_RANK} AS rank FROM topics \
                             WHERE deleted_at IS NULL \
                             AND (lower(title) LIKE ? ESCAPE '\\' \
                             OR lower(body) LIKE ? ESCAPE '\\') \
                             ORDER BY {by} LIMIT ?"
                        ))
                        .expect("prepare search topics");
                    let mapped = stmt
                        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                            let rank: i32 = row.get("rank").expect("read rank");
                            Ok((rank, topic_row(row)))
                        })
                        .expect("query search topics");
                    mapped.map(|r| r.expect("read topic")).collect()
                })
                .await
        } else {
            Vec::new()
        };

        let comment_rows: Vec<CommentRow> = if criteria.scope().covers_comments() {
            let by = match order {
                Order::Oldest => "created_at ASC",
                _ => "created_at DESC",
            };
            let params = vec![Value::Text(pattern), Value::Integer(LIMIT)];
            self.db
                .call(move |conn| {
                    let mut stmt = conn
                        .prepare(&format!(
                            "SELECT {COMMENT_COLUMNS} FROM comments WHERE deleted_at IS NULL \
                             AND lower(body) LIKE ? ESCAPE '\\' ORDER BY {by} LIMIT ?"
                        ))
                        .expect("prepare search comments");
                    let mapped = stmt
                        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                            Ok(comment_row(row))
                        })
                        .expect("query search comments");
                    mapped.map(|r| r.expect("read comment")).collect()
                })
                .await
        } else {
            Vec::new()
        };

        let mut ranked = Vec::with_capacity(topic_rows.len() + comment_rows.len());
        for (rank, row) in topic_rows {
            let tags = tags_for(&self.db, row.id).await;
            ranked.push((rank, ContentItem::Topic(to_topic(row, tags))));
        }
        for row in comment_rows {
            ranked.push((1, ContentItem::Comment(to_comment(row))));
        }
        arrange(order, ranked)
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
