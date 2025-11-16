use app::MailTokenRepository;
use domain::{MailToken, MailTokenId, TokenPurpose, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgMailTokenRepository {
    pool: PgPool,
}

impl PgMailTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, user_id, purpose, digest, payload, expires_at, redeemed_at";

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    purpose: String,
    digest: String,
    payload: Option<String>,
    expires_at: OffsetDateTime,
    redeemed_at: Option<OffsetDateTime>,
}

fn to_token(row: Row) -> MailToken {
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
impl MailTokenRepository for PgMailTokenRepository {
    async fn save(&self, token: &MailToken) {
        sqlx::query(
            "INSERT INTO mail_tokens \
             (id, user_id, purpose, digest, payload, expires_at, redeemed_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO UPDATE SET redeemed_at = EXCLUDED.redeemed_at",
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
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM mail_tokens WHERE digest = $1"
        ))
        .bind(digest)
        .fetch_optional(&self.pool)
        .await
        .expect("query mail token")
        .map(to_token)
    }
}
