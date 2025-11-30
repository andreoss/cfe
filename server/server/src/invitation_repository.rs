use app::InvitationRepository;
use domain::{Invitation, InvitationCode, InvitationId, Page, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgInvitationRepository {
    pool: PgPool,
}

impl PgInvitationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, code, issuer_id, created_at, expires_at, spent_at, spent_by";

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    code: String,
    issuer_id: uuid::Uuid,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
    spent_at: Option<OffsetDateTime>,
    spent_by: Option<uuid::Uuid>,
}

fn to_invitation(row: Row) -> Option<Invitation> {
    InvitationCode::parse(&row.code).ok().map(|code| {
        Invitation::from_parts(
            InvitationId::new(row.id),
            code,
            UserId::new(row.issuer_id),
            row.created_at,
            row.expires_at,
            row.spent_at,
            row.spent_by.map(UserId::new),
        )
    })
}

#[async_trait::async_trait]
impl InvitationRepository for PgInvitationRepository {
    async fn save(&self, invitation: &Invitation) {
        sqlx::query(
            "INSERT INTO invitations \
             (id, code, issuer_id, created_at, expires_at, spent_at, spent_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO UPDATE SET \
             code = EXCLUDED.code, \
             issuer_id = EXCLUDED.issuer_id, \
             created_at = EXCLUDED.created_at, \
             expires_at = EXCLUDED.expires_at, \
             spent_at = EXCLUDED.spent_at, \
             spent_by = EXCLUDED.spent_by",
        )
        .bind(invitation.id().as_uuid())
        .bind(invitation.code().as_str())
        .bind(invitation.issuer_id().as_uuid())
        .bind(invitation.created_at())
        .bind(invitation.expires_at())
        .bind(invitation.spent_at())
        .bind(invitation.spent_by().map(|by| by.as_uuid()))
        .execute(&self.pool)
        .await
        .expect("insert invitation");
    }

    async fn claim(&self, id: InvitationId, at: OffsetDateTime) -> bool {
        let updated =
            sqlx::query("UPDATE invitations SET spent_at = $1 WHERE id = $2 AND spent_at IS NULL")
                .bind(at)
                .bind(id.as_uuid())
                .execute(&self.pool)
                .await
                .expect("claim invitation");
        updated.rows_affected() == 1
    }

    async fn attribute(&self, id: InvitationId, by: UserId) {
        sqlx::query("UPDATE invitations SET spent_by = $1 WHERE id = $2")
            .bind(by.as_uuid())
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("attribute invitation");
    }

    async fn release(&self, id: InvitationId) {
        sqlx::query("UPDATE invitations SET spent_at = NULL, spent_by = NULL WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("release invitation");
    }

    async fn find_by_code(&self, code: &InvitationCode) -> Option<Invitation> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM invitations WHERE code = $1"
        ))
        .bind(code.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query invitation")
        .and_then(to_invitation)
    }

    async fn list_by_issuer(&self, issuer_id: UserId, page: Page) -> Vec<Invitation> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM invitations \
             WHERE issuer_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(issuer_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query invitations")
        .into_iter()
        .filter_map(to_invitation)
        .collect()
    }

    async fn count_by_issuer(&self, issuer_id: UserId) -> u64 {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM invitations WHERE issuer_id = $1")
                .bind(issuer_id.as_uuid())
                .fetch_one(&self.pool)
                .await
                .expect("count invitations");
        count as u64
    }

    async fn count_outstanding(&self, issuer_id: UserId, now: OffsetDateTime) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM invitations \
             WHERE issuer_id = $1 AND spent_at IS NULL AND expires_at > $2",
        )
        .bind(issuer_id.as_uuid())
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .expect("count outstanding invitations");
        count as u64
    }
}
