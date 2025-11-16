use app::EnforcementRepository;
use domain::{Ban, Reason, UserId, Warning, WarningId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgEnforcementRepository {
    pool: PgPool,
}

impl PgEnforcementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct BanRow {
    moderator_id: uuid::Uuid,
    reason: String,
    banned_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
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
impl EnforcementRepository for PgEnforcementRepository {
    async fn save_ban(&self, user_id: UserId, ban: &Ban) {
        sqlx::query(
            "INSERT INTO bans (user_id, moderator_id, reason, banned_at, until) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (user_id) DO UPDATE SET moderator_id = EXCLUDED.moderator_id, \
             reason = EXCLUDED.reason, banned_at = EXCLUDED.banned_at, until = EXCLUDED.until",
        )
        .bind(user_id.as_uuid())
        .bind(ban.moderator_id().as_uuid())
        .bind(ban.reason().as_str())
        .bind(ban.banned_at())
        .bind(ban.until())
        .execute(&self.pool)
        .await
        .expect("insert ban");
    }

    async fn find_ban(&self, user_id: UserId) -> Option<Ban> {
        let row = sqlx::query_as::<_, BanRow>(
            "SELECT moderator_id, reason, banned_at, until FROM bans WHERE user_id = $1",
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query ban")?;
        Some(Ban::new(
            UserId::new(row.moderator_id),
            Reason::parse(&row.reason).expect("stored reason is valid"),
            row.banned_at,
            row.until,
        ))
    }

    async fn delete_ban(&self, user_id: UserId) {
        sqlx::query("DELETE FROM bans WHERE user_id = $1")
            .bind(user_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete ban");
    }

    async fn save_warning(&self, warning: &Warning) {
        sqlx::query(
            "INSERT INTO warnings (id, user_id, moderator_id, reason, created_at, acknowledged) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (id) DO UPDATE SET acknowledged = EXCLUDED.acknowledged",
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
             FROM warnings WHERE user_id = $1 ORDER BY created_at DESC",
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
        sqlx::query(
            "INSERT INTO ignores (user_id, ignored_id) VALUES ($1, $2) \
             ON CONFLICT (user_id, ignored_id) DO NOTHING",
        )
        .bind(user_id.as_uuid())
        .bind(ignored_id.as_uuid())
        .execute(&self.pool)
        .await
        .expect("insert ignore");
    }

    async fn delete_ignore(&self, user_id: UserId, ignored_id: UserId) {
        sqlx::query("DELETE FROM ignores WHERE user_id = $1 AND ignored_id = $2")
            .bind(user_id.as_uuid())
            .bind(ignored_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete ignore");
    }

    async fn list_ignored(&self, user_id: UserId) -> Vec<UserId> {
        sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT ignored_id FROM ignores WHERE user_id = $1",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query ignores")
        .into_iter()
        .map(UserId::new)
        .collect()
    }
}
