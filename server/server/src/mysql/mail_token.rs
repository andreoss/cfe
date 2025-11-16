use app::MailTokenRepository;
use domain::{MailToken, MailTokenId, TokenPurpose, UserId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlMailTokenRepository {
    pool: MySqlPool,
}

impl MySqlMailTokenRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, user_id, purpose, digest, payload, expires_at, redeemed_at";

#[derive(FromRow)]
struct MailTokenRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    purpose: String,
    digest: String,
    payload: Option<String>,
    expires_at: OffsetDateTime,
    redeemed_at: Option<OffsetDateTime>,
}

fn to_token(row: MailTokenRow) -> MailToken {
    MailToken::from_parts(
        MailTokenId::new(row.id),
        UserId::new(row.user_id),
        TokenPurpose::parse(&row.purpose).expect("stored purpose is valid"),
        row.digest,
        row.payload,
        row.expires_at,
        row.redeemed_at,
    )
}

#[async_trait::async_trait]
impl MailTokenRepository for MySqlMailTokenRepository {
    async fn save(&self, token: &MailToken) {
        sqlx::query(
            "INSERT INTO mail_tokens \
             (id, user_id, purpose, digest, payload, expires_at, redeemed_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE redeemed_at = VALUES(redeemed_at)",
        )
        .bind(token.id().as_uuid())
        .bind(token.user_id().as_uuid())
        .bind(token.purpose().as_str())
        .bind(token.digest())
        .bind(token.payload())
        .bind(token.expires_at())
        .bind(token.redeemed_at())
        .execute(&self.pool)
        .await
        .expect("insert mail token");
    }

    async fn find_by_digest(&self, digest: &str) -> Option<MailToken> {
        sqlx::query_as::<_, MailTokenRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM mail_tokens WHERE digest = ?"
        ))
        .bind(digest)
        .fetch_optional(&self.pool)
        .await
        .expect("query mail token")
        .map(to_token)
    }
}
