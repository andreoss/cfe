use app::SessionRepository;
use domain::{Session, SessionId, SessionToken, UserId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlSessionRepository {
    pool: MySqlPool,
}

impl MySqlSessionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct SessionRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    token: String,
    expires_at: OffsetDateTime,
}

fn to_session(row: SessionRow) -> Session {
    Session::new(
        SessionId::new(row.id),
        UserId::new(row.user_id),
        SessionToken::parse(&row.token).expect("stored token is valid"),
        row.expires_at,
    )
}

#[async_trait::async_trait]
impl SessionRepository for MySqlSessionRepository {
    async fn save(&self, session: &Session) {
        sqlx::query("INSERT INTO sessions (id, user_id, token, expires_at) VALUES (?, ?, ?, ?)")
            .bind(session.id().as_uuid())
            .bind(session.user_id().as_uuid())
            .bind(session.token().as_str())
            .bind(session.expires_at())
            .execute(&self.pool)
            .await
            .expect("insert session");
    }

    async fn find_by_token(&self, token: &SessionToken) -> Option<Session> {
        sqlx::query_as::<_, SessionRow>(
            "SELECT id, user_id, token, expires_at FROM sessions WHERE token = ?",
        )
        .bind(token.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_token")
        .map(to_session)
    }

    async fn touch(&self, id: SessionId, new_expiry: OffsetDateTime) {
        sqlx::query("UPDATE sessions SET expires_at = ? WHERE id = ?")
            .bind(new_expiry)
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("touch session");
    }

    async fn delete(&self, id: SessionId) {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete session");
    }

    async fn delete_for_user(&self, user_id: UserId) {
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete sessions for user");
    }
}
