use app::AbuseRepository;
use domain::{
    Address, AddressBlock, AddressPost, BlockMode, ClientString, CommentId, Page, PostRef, Reason,
    TopicId, UserId,
};
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
    mode: String,
}

#[derive(FromRow)]
struct BlockListRow {
    addr: String,
    moderator_id: uuid::Uuid,
    reason: String,
    blocked_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
    mode: String,
}

#[derive(FromRow)]
struct AddressPostRow {
    user_id: uuid::Uuid,
    client: Option<String>,
    created_at: OffsetDateTime,
}

#[derive(FromRow)]
struct TargetRow {
    topic_id: Option<uuid::Uuid>,
    comment_id: Option<uuid::Uuid>,
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
            "SELECT moderator_id, reason, blocked_at, until, mode FROM address_blocks \
             WHERE addr = ?",
        )
        .bind(addr.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query address block")?;
        Some(
            AddressBlock::new(
                addr.clone(),
                UserId::new(row.moderator_id),
                Reason::parse(&row.reason).expect("stored reason is valid"),
                row.blocked_at,
                row.until,
            )
            .with_mode(BlockMode::parse(&row.mode).unwrap_or_default()),
        )
    }

    async fn save_address_block(&self, addr: &Address, block: &AddressBlock) {
        sqlx::query(
            "INSERT INTO address_blocks (addr, moderator_id, reason, blocked_at, until, mode) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE moderator_id = VALUES(moderator_id), \
             reason = VALUES(reason), blocked_at = VALUES(blocked_at), \
             until = VALUES(until), mode = VALUES(mode)",
        )
        .bind(addr.as_str())
        .bind(block.moderator_id().as_uuid())
        .bind(block.reason().as_str())
        .bind(block.blocked_at())
        .bind(block.until())
        .bind(block.mode().as_str())
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
            "SELECT addr, moderator_id, reason, blocked_at, until, mode FROM address_blocks",
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
                .with_mode(BlockMode::parse(&row.mode).unwrap_or_default())
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

    async fn count_posts_by_user(&self, user_id: UserId, since: OffsetDateTime) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM post_events WHERE subject = ? AND created_at >= ?",
        )
        .bind(slow_subject(user_id))
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .expect("query account rate count");
        count as u64
    }

    async fn last_post_by_user(&self, user_id: UserId) -> Option<OffsetDateTime> {
        sqlx::query_scalar("SELECT MAX(created_at) FROM post_events WHERE subject = ?")
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
        target: Option<PostRef>,
        at: OffsetDateTime,
    ) {
        let client = client.map(|c| c.as_str().to_owned());
        let topic_id = target.and_then(|t| t.topic_id()).map(|id| id.as_uuid());
        let comment_id = target.and_then(|t| t.comment_id()).map(|id| id.as_uuid());
        let mut tx = self.pool.begin().await.expect("begin record post");
        sqlx::query(
            "INSERT INTO post_events \
             (subject, kind, user_id, client, topic_id, comment_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(rate_subject(addr))
        .bind("rate")
        .bind(user_id.as_uuid())
        .bind(client.as_deref())
        .bind(topic_id)
        .bind(comment_id)
        .bind(at)
        .execute(&mut *tx)
        .await
        .expect("insert rate event");
        sqlx::query(
            "INSERT INTO post_events \
             (subject, kind, user_id, client, topic_id, comment_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(slow_subject(user_id))
        .bind("slow")
        .bind(user_id.as_uuid())
        .bind(client.as_deref())
        .bind(topic_id)
        .bind(comment_id)
        .bind(at)
        .execute(&mut *tx)
        .await
        .expect("insert slow event");
        tx.commit().await.expect("commit record post");
    }

    async fn refs_from_address_since(&self, addr: &Address, since: OffsetDateTime) -> Vec<PostRef> {
        let rows = sqlx::query_as::<_, TargetRow>(
            "SELECT topic_id, comment_id FROM post_events \
             WHERE subject = ? AND created_at >= ? \
             AND (topic_id IS NOT NULL OR comment_id IS NOT NULL) \
             ORDER BY created_at DESC",
        )
        .bind(rate_subject(addr))
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .expect("query address refs");
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
        sqlx::query("INSERT INTO sign_in_failures (username, addr, created_at) VALUES (?, ?, ?)")
            .bind(username)
            .bind(addr.as_str())
            .bind(at)
            .execute(&self.pool)
            .await
            .expect("insert sign in failure");
    }

    async fn count_sign_in_failures(
        &self,
        username: &str,
        addr: &Address,
        since: OffsetDateTime,
    ) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sign_in_failures \
             WHERE username = ? AND addr = ? AND created_at >= ?",
        )
        .bind(username)
        .bind(addr.as_str())
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .expect("query sign in failures");
        count as u64
    }

    async fn clear_sign_in_failures(&self, username: &str) {
        sqlx::query("DELETE FROM sign_in_failures WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await
            .expect("delete sign in failures");
    }

    async fn has_seen_address(&self, user_id: UserId, addr: &Address) -> bool {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM known_addresses WHERE user_id = ? AND addr = ?",
        )
        .bind(user_id.as_uuid())
        .bind(addr.as_str())
        .fetch_one(&self.pool)
        .await
        .expect("query known address");
        count > 0
    }

    async fn remember_address(&self, user_id: UserId, addr: &Address, at: OffsetDateTime) {
        sqlx::query(
            "INSERT INTO known_addresses (user_id, addr, first_seen) VALUES (?, ?, ?) \
             ON DUPLICATE KEY UPDATE first_seen = first_seen",
        )
        .bind(user_id.as_uuid())
        .bind(addr.as_str())
        .bind(at)
        .execute(&self.pool)
        .await
        .expect("insert known address");
    }

    async fn posts_from_address(&self, addr: &Address, page: Page) -> Vec<AddressPost> {
        let rows = sqlx::query_as::<_, AddressPostRow>(
            "SELECT user_id, client, created_at FROM post_events \
             WHERE subject = ? AND user_id IS NOT NULL \
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
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
            "SELECT COUNT(*) FROM post_events WHERE subject = ? AND user_id IS NOT NULL",
        )
        .bind(rate_subject(addr))
        .fetch_one(&self.pool)
        .await
        .expect("query address post count");
        count as u64
    }
}
