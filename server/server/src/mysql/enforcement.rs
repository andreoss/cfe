use app::EnforcementRepository;
use domain::{Ban, Reason, UserId, Warning, WarningId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlEnforcementRepository {
    pool: MySqlPool,
}

impl MySqlEnforcementRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct BanRow {
    moderator_id: Option<uuid::Uuid>,
    reason: String,
    banned_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
}

fn stored_moderator(id: UserId) -> Option<uuid::Uuid> {
    if id.is_scheduled_work() {
        None
    } else {
        Some(id.as_uuid())
    }
}

#[derive(FromRow)]
struct WarningRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    moderator_id: uuid::Uuid,
    reason: String,
    created_at: OffsetDateTime,
    acknowledged: bool,
}

#[async_trait::async_trait]
impl EnforcementRepository for MySqlEnforcementRepository {
    async fn save_ban(&self, user_id: UserId, ban: &Ban) {
        sqlx::query(
            "INSERT INTO bans (user_id, moderator_id, reason, banned_at, until) \
             VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE moderator_id = VALUES(moderator_id), \
             reason = VALUES(reason), banned_at = VALUES(banned_at), until = VALUES(until)",
        )
        .bind(user_id.as_uuid())
        .bind(stored_moderator(ban.moderator_id()))
        .bind(ban.reason().as_str())
        .bind(ban.banned_at())
        .bind(ban.until())
        .execute(&self.pool)
        .await
        .expect("insert ban");
    }

    async fn find_ban(&self, user_id: UserId) -> Option<Ban> {
        let row = sqlx::query_as::<_, BanRow>(
            "SELECT moderator_id, reason, banned_at, until FROM bans WHERE user_id = ?",
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query ban")?;
        Some(Ban::new(
            row.moderator_id
                .map(UserId::new)
                .unwrap_or_else(UserId::scheduled_work),
            Reason::parse(&row.reason).expect("stored reason is valid"),
            row.banned_at,
            row.until,
        ))
    }

    async fn delete_ban(&self, user_id: UserId) {
        sqlx::query("DELETE FROM bans WHERE user_id = ?")
            .bind(user_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete ban");
    }

    async fn save_warning(&self, warning: &Warning) {
        sqlx::query(
            "INSERT INTO warnings (id, user_id, moderator_id, reason, created_at, acknowledged) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE acknowledged = VALUES(acknowledged)",
        )
        .bind(warning.id().as_uuid())
        .bind(warning.user_id().as_uuid())
        .bind(warning.moderator_id().as_uuid())
        .bind(warning.reason().as_str())
        .bind(warning.created_at())
        .bind(warning.is_acknowledged())
        .execute(&self.pool)
        .await
        .expect("insert warning");
    }

    async fn list_warnings(&self, user_id: UserId) -> Vec<Warning> {
        sqlx::query_as::<_, WarningRow>(
            "SELECT id, user_id, moderator_id, reason, created_at, acknowledged \
             FROM warnings WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query warnings")
        .into_iter()
        .map(|row| {
            Warning::from_parts(
                WarningId::new(row.id),
                UserId::new(row.user_id),
                UserId::new(row.moderator_id),
                Reason::parse(&row.reason).expect("stored reason is valid"),
                row.created_at,
                row.acknowledged,
            )
        })
        .collect()
    }

    async fn save_ignore(&self, user_id: UserId, ignored_id: UserId) {
        sqlx::query("INSERT IGNORE INTO ignores (user_id, ignored_id) VALUES (?, ?)")
            .bind(user_id.as_uuid())
            .bind(ignored_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("insert ignore");
    }

    async fn delete_ignore(&self, user_id: UserId, ignored_id: UserId) {
        sqlx::query("DELETE FROM ignores WHERE user_id = ? AND ignored_id = ?")
            .bind(user_id.as_uuid())
            .bind(ignored_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete ignore");
    }

    async fn list_ignored(&self, user_id: UserId) -> Vec<UserId> {
        sqlx::query_scalar::<_, uuid::Uuid>("SELECT ignored_id FROM ignores WHERE user_id = ?")
            .bind(user_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .expect("query ignores")
            .into_iter()
            .map(UserId::new)
            .collect()
    }
}
