use crate::duckdb::conn::Db;
use crate::duckdb::topic::{
    opt_time, read_opt_time, read_time, read_uuid, time_to_value, uuid_value,
};
use app::AbuseRepository;
use domain::{Address, AddressBlock, UserId};
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckAbuseRepository {
    db: Db,
}

impl DuckAbuseRepository {
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
}

#[async_trait::async_trait]
impl AbuseRepository for DuckAbuseRepository {
    async fn find_address_block(&self, addr: &Address) -> Option<AddressBlock> {
        let subject = addr.as_str().to_owned();
        let rows: Vec<BlockRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT moderator_id, reason, blocked_at, until FROM address_blocks \
                         WHERE addr = ?",
                    )
                    .expect("prepare address block");
                let mapped = stmt
                    .query_map([Value::Text(subject)], |row| {
                        Ok(BlockRow {
                            moderator_id: read_uuid(row, 0),
                            reason: row.get(1).expect("read reason"),
                            blocked_at: read_time(row, 2),
                            until: read_opt_time(row, 3),
                        })
                    })
                    .expect("query address block");
                mapped.map(|r| r.expect("read block")).collect()
            })
            .await;
        let row = rows.into_iter().next()?;
        Some(AddressBlock::new(
            addr.clone(),
            UserId::new(row.moderator_id),
            domain::Reason::parse(&row.reason).expect("stored reason is valid"),
            row.blocked_at,
            row.until,
        ))
    }

    async fn save_address_block(&self, addr: &Address, block: &AddressBlock) {
        let params = vec![
            Value::Text(addr.as_str().to_owned()),
            uuid_value(block.moderator_id().as_uuid()),
            Value::Text(block.reason().as_str().to_owned()),
            time_to_value(block.blocked_at()),
            opt_time(block.until()),
        ];
        self.db
            .execute(
                "INSERT INTO address_blocks (addr, moderator_id, reason, blocked_at, until) \
                 VALUES (?, ?, ?, ?, ?) \
                 ON CONFLICT (addr) DO UPDATE SET moderator_id = EXCLUDED.moderator_id, \
                 reason = EXCLUDED.reason, blocked_at = EXCLUDED.blocked_at, until = EXCLUDED.until",
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
                        "SELECT addr, moderator_id, reason, blocked_at, until FROM address_blocks",
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

    async fn record_post(&self, user_id: UserId, addr: &Address, at: OffsetDateTime) {
        self.db
            .execute(
                "INSERT INTO post_events (subject, kind, user_id, created_at) VALUES (?, ?, ?, ?)",
                vec![
                    Value::Text(rate_subject(addr)),
                    Value::Text("rate".to_owned()),
                    Value::Null,
                    time_to_value(at),
                ],
            )
            .await;
        self.db
            .execute(
                "INSERT INTO post_events (subject, kind, user_id, created_at) VALUES (?, ?, ?, ?)",
                vec![
                    Value::Text(slow_subject(user_id)),
                    Value::Text("slow".to_owned()),
                    Value::Null,
                    time_to_value(at),
                ],
            )
            .await;
    }
}
