use crate::sqlite::conn::Db;
use crate::sqlite::topic::{
    count, limit_value, offset_value, opt_text, opt_time, opt_uuid, read_opt_time, read_opt_uuid,
    read_time, read_uuid, time_to_value, uuid_value,
};
use app::AbuseRepository;
use domain::{
    Address, AddressBlock, AddressPost, BlockMode, ClientString, CommentId, Page, PostRef, TopicId,
    UserId,
};
use rusqlite::types::Value;
use time::OffsetDateTime;

pub struct SqliteAbuseRepository {
    db: Db,
}

impl SqliteAbuseRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

fn rate_subject(addr: &Address) -> String {
    format!("addr:{}", addr.as_str())
}

fn slow_subject(user_id: UserId) -> String {
    format!("user:{}", user_id.as_uuid())
}

struct BlockRow {
    moderator_id: uuid::Uuid,
    reason: String,
    blocked_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
    mode: String,
}

struct PostRow {
    user_id: uuid::Uuid,
    client: Option<String>,
    created_at: OffsetDateTime,
}

struct TargetRow {
    topic_id: Option<uuid::Uuid>,
    comment_id: Option<uuid::Uuid>,
}

#[async_trait::async_trait]
impl AbuseRepository for SqliteAbuseRepository {
    async fn find_address_block(&self, addr: &Address) -> Option<AddressBlock> {
        let subject = addr.as_str().to_owned();
        let rows: Vec<BlockRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT moderator_id, reason, blocked_at, until, mode \
                         FROM address_blocks WHERE addr = ?",
                    )
                    .expect("prepare address block");
                let mapped = stmt
                    .query_map([Value::Text(subject)], |row| {
                        Ok(BlockRow {
                            moderator_id: read_uuid(row, 0),
                            reason: row.get(1).expect("read reason"),
                            blocked_at: read_time(row, 2),
                            until: read_opt_time(row, 3),
                            mode: row.get(4).expect("read mode"),
                        })
                    })
                    .expect("query address block");
                mapped.map(|r| r.expect("read block")).collect()
            })
            .await;
        let row = rows.into_iter().next()?;
        Some(
            AddressBlock::new(
                addr.clone(),
                UserId::new(row.moderator_id),
                domain::Reason::parse(&row.reason).expect("stored reason is valid"),
                row.blocked_at,
                row.until,
            )
            .with_mode(BlockMode::parse(&row.mode).unwrap_or_default()),
        )
    }

    async fn save_address_block(&self, addr: &Address, block: &AddressBlock) {
        let params = vec![
            Value::Text(addr.as_str().to_owned()),
            uuid_value(block.moderator_id().as_uuid()),
            Value::Text(block.reason().as_str().to_owned()),
            time_to_value(block.blocked_at()),
            opt_time(block.until()),
            Value::Text(block.mode().as_str().to_owned()),
        ];
        self.db
            .execute(
                "INSERT INTO address_blocks (addr, moderator_id, reason, blocked_at, until, mode) \
                 VALUES (?, ?, ?, ?, ?, ?) \
                 ON CONFLICT (addr) DO UPDATE SET moderator_id = EXCLUDED.moderator_id, \
                 reason = EXCLUDED.reason, blocked_at = EXCLUDED.blocked_at, \
                 until = EXCLUDED.until, mode = EXCLUDED.mode",
                params,
            )
            .await;
    }

    async fn delete_address_block(&self, addr: &Address) {
        self.db
            .execute(
                "DELETE FROM address_blocks WHERE addr = ?",
                vec![Value::Text(addr.as_str().to_owned())],
            )
            .await;
    }

    async fn list_address_blocks(&self) -> Vec<AddressBlock> {
        let rows: Vec<(String, BlockRow)> = self
            .db
            .call(|conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT addr, moderator_id, reason, blocked_at, until, mode \
                         FROM address_blocks",
                    )
                    .expect("prepare address blocks");
                let mapped = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0).expect("read addr"),
                            BlockRow {
                                moderator_id: read_uuid(row, 1),
                                reason: row.get(2).expect("read reason"),
                                blocked_at: read_time(row, 3),
                                until: read_opt_time(row, 4),
                                mode: row.get(5).expect("read mode"),
                            },
                        ))
                    })
                    .expect("query address blocks");
                mapped.map(|r| r.expect("read block")).collect()
            })
            .await;
        rows.into_iter()
            .map(|(addr, row)| {
                AddressBlock::new(
                    Address::parse(&addr).expect("stored address is valid"),
                    UserId::new(row.moderator_id),
                    domain::Reason::parse(&row.reason).expect("stored reason is valid"),
                    row.blocked_at,
                    row.until,
                )
                .with_mode(BlockMode::parse(&row.mode).unwrap_or_default())
            })
            .collect()
    }

    async fn count_posts_by_address(&self, addr: &Address, since: OffsetDateTime) -> u64 {
        let subject = rate_subject(addr);
        let params = [Value::Text(subject), time_to_value(since)];
        self.db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT COUNT(*) FROM post_events WHERE subject = ? AND created_at >= ?",
                    )
                    .expect("prepare rate count");
                let count: i64 = stmt
                    .query_row(params, |row| row.get(0))
                    .expect("query rate count");
                count as u64
            })
            .await
    }

    async fn count_posts_by_user(&self, user_id: UserId, since: OffsetDateTime) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM post_events WHERE subject = ? AND created_at >= ?",
            vec![Value::Text(slow_subject(user_id)), time_to_value(since)],
        )
        .await
    }

    async fn last_post_by_user(&self, user_id: UserId) -> Option<OffsetDateTime> {
        let subject = slow_subject(user_id);
        self.db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT MAX(created_at) FROM post_events WHERE subject = ?")
                    .expect("prepare last post");
                let at: Option<OffsetDateTime> = stmt
                    .query_row([Value::Text(subject)], |row| Ok(read_opt_time(row, 0)))
                    .expect("query last post");
                at
            })
            .await
    }

    async fn record_post(
        &self,
        user_id: UserId,
        addr: &Address,
        client: Option<&ClientString>,
        target: Option<PostRef>,
        at: OffsetDateTime,
    ) {
        let client = client.map(|c| c.as_str());
        let topic_id = target.and_then(|t| t.topic_id()).map(|id| id.as_uuid());
        let comment_id = target.and_then(|t| t.comment_id()).map(|id| id.as_uuid());
        self.db
            .execute(
                "INSERT INTO post_events \
                 (subject, kind, user_id, client, topic_id, comment_id, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                vec![
                    Value::Text(rate_subject(addr)),
                    Value::Text("rate".to_owned()),
                    uuid_value(user_id.as_uuid()),
                    opt_text(client),
                    opt_uuid(topic_id),
                    opt_uuid(comment_id),
                    time_to_value(at),
                ],
            )
            .await;
        self.db
            .execute(
                "INSERT INTO post_events \
                 (subject, kind, user_id, client, topic_id, comment_id, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                vec![
                    Value::Text(slow_subject(user_id)),
                    Value::Text("slow".to_owned()),
                    uuid_value(user_id.as_uuid()),
                    opt_text(client),
                    opt_uuid(topic_id),
                    opt_uuid(comment_id),
                    time_to_value(at),
                ],
            )
            .await;
    }

    async fn refs_from_address_since(&self, addr: &Address, since: OffsetDateTime) -> Vec<PostRef> {
        let params = vec![Value::Text(rate_subject(addr)), time_to_value(since)];
        let rows: Vec<TargetRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT topic_id, comment_id FROM post_events \
                         WHERE subject = ? AND created_at >= ? \
                         AND (topic_id IS NOT NULL OR comment_id IS NOT NULL) \
                         ORDER BY created_at DESC",
                    )
                    .expect("prepare address refs");
                let mapped = stmt
                    .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                        Ok(TargetRow {
                            topic_id: read_opt_uuid(row, 0),
                            comment_id: read_opt_uuid(row, 1),
                        })
                    })
                    .expect("query address refs");
                mapped.map(|r| r.expect("read address ref")).collect()
            })
            .await;
        rows.into_iter()
            .filter_map(|row| {
                PostRef::from_parts(
                    row.topic_id.map(TopicId::new),
                    row.comment_id.map(CommentId::new),
                )
            })
            .collect()
    }

    async fn record_sign_in_failure(&self, username: &str, addr: &Address, at: OffsetDateTime) {
        self.db
            .execute(
                "INSERT INTO sign_in_failures (username, addr, created_at) VALUES (?, ?, ?)",
                vec![
                    Value::Text(username.to_owned()),
                    Value::Text(addr.as_str().to_owned()),
                    time_to_value(at),
                ],
            )
            .await;
    }

    async fn count_sign_in_failures(
        &self,
        username: &str,
        addr: &Address,
        since: OffsetDateTime,
    ) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM sign_in_failures \
             WHERE username = ? AND addr = ? AND created_at >= ?",
            vec![
                Value::Text(username.to_owned()),
                Value::Text(addr.as_str().to_owned()),
                time_to_value(since),
            ],
        )
        .await
    }

    async fn clear_sign_in_failures(&self, username: &str) {
        self.db
            .execute(
                "DELETE FROM sign_in_failures WHERE username = ?",
                vec![Value::Text(username.to_owned())],
            )
            .await;
    }

    async fn has_seen_address(&self, user_id: UserId, addr: &Address) -> bool {
        count(
            &self.db,
            "SELECT COUNT(*) FROM known_addresses WHERE user_id = ? AND addr = ?",
            vec![
                uuid_value(user_id.as_uuid()),
                Value::Text(addr.as_str().to_owned()),
            ],
        )
        .await
            > 0
    }

    async fn remember_address(&self, user_id: UserId, addr: &Address, at: OffsetDateTime) {
        self.db
            .execute(
                "INSERT INTO known_addresses (user_id, addr, first_seen) VALUES (?, ?, ?) \
                 ON CONFLICT (user_id, addr) DO NOTHING",
                vec![
                    uuid_value(user_id.as_uuid()),
                    Value::Text(addr.as_str().to_owned()),
                    time_to_value(at),
                ],
            )
            .await;
    }

    async fn posts_from_address(&self, addr: &Address, page: Page) -> Vec<AddressPost> {
        let params = vec![
            Value::Text(rate_subject(addr)),
            limit_value(page),
            offset_value(page),
        ];
        let rows: Vec<PostRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT user_id, client, created_at FROM post_events \
                         WHERE subject = ? AND user_id IS NOT NULL \
                         ORDER BY created_at DESC LIMIT ? OFFSET ?",
                    )
                    .expect("prepare address posts");
                let mapped = stmt
                    .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                        Ok(PostRow {
                            user_id: read_uuid(row, 0),
                            client: row.get(1).expect("read client"),
                            created_at: read_time(row, 2),
                        })
                    })
                    .expect("query address posts");
                mapped.map(|r| r.expect("read address post")).collect()
            })
            .await;
        rows.into_iter()
            .map(|row| {
                AddressPost::new(
                    UserId::new(row.user_id),
                    addr.clone(),
                    row.client.and_then(|c| ClientString::parse(&c).ok()),
                    row.created_at,
                )
            })
            .collect()
    }

    async fn count_posts_from_address(&self, addr: &Address) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM post_events WHERE subject = ? AND user_id IS NOT NULL",
            vec![Value::Text(rate_subject(addr))],
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Reason;

    const SCHEMA: &str = include_str!("../../migrations_sqlite/0001_schema.sql");

    async fn repository() -> SqliteAbuseRepository {
        let db = Db::open(":memory:");
        db.call(|conn| {
            conn.execute_batch(SCHEMA).expect("run schema");
        })
        .await;
        SqliteAbuseRepository::new(db)
    }

    fn block(mode: BlockMode) -> AddressBlock {
        AddressBlock::new(
            Address::parse("203.0.113.10").unwrap(),
            UserId::new(uuid::Uuid::nil()),
            Reason::parse("a shared address").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        )
        .with_mode(mode)
    }

    #[tokio::test]
    async fn a_stored_block_keeps_its_mode() {
        let repo = repository().await;
        let addr = Address::parse("203.0.113.10").unwrap();
        repo.save_address_block(&addr, &block(BlockMode::Challenge))
            .await;
        assert_eq!(
            repo.find_address_block(&addr).await.map(|b| b.mode()),
            Some(BlockMode::Challenge)
        );
        assert_eq!(
            repo.list_address_blocks().await.first().map(|b| b.mode()),
            Some(BlockMode::Challenge)
        );
    }

    #[tokio::test]
    async fn failures_are_counted_for_one_name_and_address_within_a_window() {
        let repo = repository().await;
        let addr = Address::parse("203.0.113.10").unwrap();
        let other = Address::parse("198.51.100.4").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        repo.record_sign_in_failure("owner_01", &addr, now).await;
        repo.record_sign_in_failure("owner_01", &addr, now).await;
        repo.record_sign_in_failure("owner_01", &other, now).await;
        repo.record_sign_in_failure("quiet_01", &addr, now).await;
        assert_eq!(repo.count_sign_in_failures("owner_01", &addr, now).await, 2);
        let later = now + time::Duration::seconds(1);
        assert_eq!(
            repo.count_sign_in_failures("owner_01", &addr, later).await,
            0
        );
        repo.clear_sign_in_failures("owner_01").await;
        assert_eq!(repo.count_sign_in_failures("owner_01", &addr, now).await, 0);
        assert_eq!(repo.count_sign_in_failures("quiet_01", &addr, now).await, 1);
    }

    #[tokio::test]
    async fn an_address_may_be_remembered_twice_for_one_account() {
        let repo = repository().await;
        let addr = Address::parse("203.0.113.10").unwrap();
        let owner = UserId::new(uuid::Uuid::from_u128(1));
        let other = UserId::new(uuid::Uuid::from_u128(2));
        assert!(!repo.has_seen_address(owner, &addr).await);
        repo.remember_address(owner, &addr, OffsetDateTime::UNIX_EPOCH)
            .await;
        repo.remember_address(owner, &addr, OffsetDateTime::UNIX_EPOCH)
            .await;
        assert!(repo.has_seen_address(owner, &addr).await);
        assert!(!repo.has_seen_address(other, &addr).await);
    }

    #[tokio::test]
    async fn saving_the_same_address_again_moves_its_mode() {
        let repo = repository().await;
        let addr = Address::parse("203.0.113.10").unwrap();
        repo.save_address_block(&addr, &block(BlockMode::Refuse))
            .await;
        repo.save_address_block(&addr, &block(BlockMode::Allow))
            .await;
        assert_eq!(repo.list_address_blocks().await.len(), 1);
        assert_eq!(
            repo.find_address_block(&addr).await.map(|b| b.mode()),
            Some(BlockMode::Allow)
        );
    }
}
