use app::SessionRepository;
use domain::{Session, SessionId, SessionToken, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    token: String,
    expires_at: OffsetDateTime,
}

fn to_session(row: Row) -> Session {
    Session::new(
        SessionId::new(row.id),
        UserId::new(row.user_id),
        SessionToken::parse(&row.token).expect("stored token is valid"),
        row.expires_at,
    )
}

#[async_trait::async_trait]
impl SessionRepository for PgSessionRepository {
    async fn save(&self, session: &Session) {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token, expires_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(session.id().as_uuid())
        .bind(session.user_id().as_uuid())
        .bind(session.token().as_str())
        .bind(session.expires_at())
        .execute(&self.pool)
        .await
        .expect("insert session");
    }

    async fn find_by_token(&self, token: &SessionToken) -> Option<Session> {
        sqlx::query_as::<_, Row>(
            "SELECT id, user_id, token, expires_at FROM sessions WHERE token = $1",
        )
        .bind(token.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_token")
        .map(to_session)
    }

    async fn touch(&self, id: SessionId, new_expiry: OffsetDateTime) {
        sqlx::query("UPDATE sessions SET expires_at = $1 WHERE id = $2")
            .bind(new_expiry)
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("touch session");
    }

    async fn delete(&self, id: SessionId) {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete session");
    }

    async fn delete_for_user(&self, user_id: domain::UserId) {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete sessions for user");
    }
}
