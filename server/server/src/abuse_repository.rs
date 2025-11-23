use app::AbuseRepository;
use domain::{Address, AddressBlock, AddressPost, ClientString, Page, Reason, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgAbuseRepository {
    pool: PgPool,
}

impl PgAbuseRepository {
    pub fn new(pool: PgPool) -> Self {
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

#[derive(FromRow)]
struct AddressPostRow {
    user_id: uuid::Uuid,
    client: Option<String>,
    created_at: OffsetDateTime,
}

fn rate_subject(addr: &Address) -> String {
    format!("addr:{}", addr.as_str())
}

fn slow_subject(user_id: UserId) -> String {
    format!("user:{}", user_id.as_uuid())
}

#[async_trait::async_trait]
impl AbuseRepository for PgAbuseRepository {
    async fn find_address_block(&self, addr: &Address) -> Option<AddressBlock> {
        let row = sqlx::query_as::<_, BlockRow>(
            "SELECT moderator_id, reason, blocked_at, until FROM address_blocks WHERE addr = $1",
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
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (addr) DO UPDATE SET moderator_id = EXCLUDED.moderator_id, \
             reason = EXCLUDED.reason, blocked_at = EXCLUDED.blocked_at, until = EXCLUDED.until",
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
        sqlx::query("DELETE FROM address_blocks WHERE addr = $1")
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
            "SELECT COUNT(*) FROM post_events WHERE subject = $1 AND created_at >= $2",
        )
        .bind(rate_subject(addr))
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .expect("query rate count");
        count as u64
    }

    async fn count_posts_by_user(&self, user_id: UserId, since: OffsetDateTime) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM post_events WHERE subject = $1 AND created_at >= $2",
        )
        .bind(slow_subject(user_id))
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .expect("query account rate count");
        count as u64
    }

    async fn last_post_by_user(&self, user_id: UserId) -> Option<OffsetDateTime> {
        sqlx::query_scalar("SELECT MAX(created_at) FROM post_events WHERE subject = $1")
            .bind(slow_subject(user_id))
            .fetch_one(&self.pool)
            .await
            .expect("query last post")
    }

    async fn record_post(
        &self,
        user_id: UserId,
        addr: &Address,
        client: Option<&ClientString>,
        at: OffsetDateTime,
    ) {
        let client = client.map(|c| c.as_str().to_owned());
        let mut tx = self.pool.begin().await.expect("begin record post");
        sqlx::query(
            "INSERT INTO post_events (subject, kind, user_id, client, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(rate_subject(addr))
        .bind("rate")
        .bind(user_id.as_uuid())
        .bind(client.as_deref())
        .bind(at)
        .execute(&mut *tx)
        .await
        .expect("insert rate event");
        sqlx::query(
            "INSERT INTO post_events (subject, kind, user_id, client, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(slow_subject(user_id))
        .bind("slow")
        .bind(user_id.as_uuid())
        .bind(client.as_deref())
        .bind(at)
        .execute(&mut *tx)
        .await
        .expect("insert slow event");
        tx.commit().await.expect("commit record post");
    }

    async fn posts_from_address(&self, addr: &Address, page: Page) -> Vec<AddressPost> {
        let rows = sqlx::query_as::<_, AddressPostRow>(
            "SELECT user_id, client, created_at FROM post_events \
             WHERE subject = $1 AND user_id IS NOT NULL \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(rate_subject(addr))
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query address posts");
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
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM post_events WHERE subject = $1 AND user_id IS NOT NULL",
        )
        .bind(rate_subject(addr))
        .fetch_one(&self.pool)
        .await
        .expect("query address post count");
        count as u64
    }
}
