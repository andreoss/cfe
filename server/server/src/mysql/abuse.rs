use app::AbuseRepository;
use domain::{Address, AddressBlock, Reason, UserId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlAbuseRepository {
    pool: MySqlPool,
}

impl MySqlAbuseRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct BlockRow {
    moderator_id: uuid::Uuid,
    reason: String,
    blocked_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
}

#[derive(FromRow)]
struct BlockListRow {
    addr: String,
    moderator_id: uuid::Uuid,
    reason: String,
    blocked_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
}

fn rate_subject(addr: &Address) -> String {
    format!("addr:{}", addr.as_str())
}

fn slow_subject(user_id: UserId) -> String {
    format!("user:{}", user_id.as_uuid())
}

#[async_trait::async_trait]
impl AbuseRepository for MySqlAbuseRepository {
    async fn find_address_block(&self, addr: &Address) -> Option<AddressBlock> {
        let row = sqlx::query_as::<_, BlockRow>(
            "SELECT moderator_id, reason, blocked_at, until FROM address_blocks WHERE addr = ?",
        )
        .bind(addr.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query address block")?;
        Some(AddressBlock::new(
            addr.clone(),
            UserId::new(row.moderator_id),
            Reason::parse(&row.reason).expect("stored reason is valid"),
            row.blocked_at,
            row.until,
        ))
    }

    async fn save_address_block(&self, addr: &Address, block: &AddressBlock) {
        sqlx::query(
            "INSERT INTO address_blocks (addr, moderator_id, reason, blocked_at, until) \
             VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE moderator_id = VALUES(moderator_id), \
             reason = VALUES(reason), blocked_at = VALUES(blocked_at), until = VALUES(until)",
        )
        .bind(addr.as_str())
        .bind(block.moderator_id().as_uuid())
        .bind(block.reason().as_str())
        .bind(block.blocked_at())
        .bind(block.until())
        .execute(&self.pool)
        .await
        .expect("insert address block");
    }

    async fn delete_address_block(&self, addr: &Address) {
        sqlx::query("DELETE FROM address_blocks WHERE addr = ?")
            .bind(addr.as_str())
            .execute(&self.pool)
            .await
            .expect("delete address block");
    }

    async fn list_address_blocks(&self) -> Vec<AddressBlock> {
        let rows = sqlx::query_as::<_, BlockListRow>(
            "SELECT addr, moderator_id, reason, blocked_at, until FROM address_blocks",
        )
        .fetch_all(&self.pool)
        .await
        .expect("query address blocks");
        rows.into_iter()
            .map(|row| {
                AddressBlock::new(
                    Address::parse(&row.addr).expect("stored address is valid"),
                    UserId::new(row.moderator_id),
                    Reason::parse(&row.reason).expect("stored reason is valid"),
                    row.blocked_at,
                    row.until,
                )
            })
            .collect()
    }

    async fn count_posts_by_address(&self, addr: &Address, since: OffsetDateTime) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM post_events WHERE subject = ? AND created_at >= ?",
        )
        .bind(rate_subject(addr))
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .expect("query rate count");
        count as u64
    }

    async fn last_post_by_user(&self, user_id: UserId) -> Option<OffsetDateTime> {
        sqlx::query_scalar("SELECT MAX(created_at) FROM post_events WHERE subject = ?")
            .bind(slow_subject(user_id))
            .fetch_one(&self.pool)
            .await
            .expect("query last post")
    }

    async fn record_post(&self, user_id: UserId, addr: &Address, at: OffsetDateTime) {
        let mut tx = self
            .pool
            .begin()
            .await
            .expect("begin record post");
        sqlx::query(
            "INSERT INTO post_events (subject, kind, user_id, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(rate_subject(addr))
        .bind("rate")
        .bind(Option::<uuid::Uuid>::None)
        .bind(at)
        .execute(&mut *tx)
        .await
        .expect("insert rate event");
        sqlx::query(
            "INSERT INTO post_events (subject, kind, user_id, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(slow_subject(user_id))
        .bind("slow")
        .bind(Option::<uuid::Uuid>::None)
        .bind(at)
        .execute(&mut *tx)
        .await
        .expect("insert slow event");
        tx.commit().await.expect("commit record post");
    }
}