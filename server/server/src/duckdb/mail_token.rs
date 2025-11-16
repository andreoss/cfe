use crate::duckdb::conn::Db;
use crate::duckdb::topic::{
    opt_text, opt_time, read_opt_time, read_time, read_uuid, time_to_value, uuid_value,
};
use app::MailTokenRepository;
use domain::{MailToken, MailTokenId, TokenPurpose, UserId};
use duckdb::Row;
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckMailTokenRepository {
    db: Db,
}

impl DuckMailTokenRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const MAIL_TOKEN_COLUMNS: &str = "id, user_id, purpose, digest, payload, expires_at, redeemed_at";

struct MailTokenRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    purpose: String,
    digest: String,
    payload: Option<String>,
    expires_at: OffsetDateTime,
    redeemed_at: Option<OffsetDateTime>,
}

fn mail_token_row(row: &Row) -> MailTokenRow {
    MailTokenRow {
        id: read_uuid(row, 0),
        user_id: read_uuid(row, 1),
        purpose: row.get(2).expect("read purpose"),
        digest: row.get(3).expect("read digest"),
        payload: row.get(4).expect("read payload"),
        expires_at: read_time(row, 5),
        redeemed_at: read_opt_time(row, 6),
    }
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

async fn load_tokens(db: &Db, sql: String, params: Vec<Value>) -> Vec<MailToken> {
    let rows: Vec<MailTokenRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare mail tokens");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(mail_token_row(row))
                })
                .expect("query mail tokens");
            mapped.map(|r| r.expect("read mail token")).collect()
        })
        .await;
    rows.into_iter().map(to_token).collect()
}

#[async_trait::async_trait]
impl MailTokenRepository for DuckMailTokenRepository {
    async fn save(&self, token: &MailToken) {
        let params = vec![
            uuid_value(token.id().as_uuid()),
            uuid_value(token.user_id().as_uuid()),
            Value::Text(token.purpose().as_str().to_owned()),
            Value::Text(token.digest().to_owned()),
            opt_text(token.payload()),
            time_to_value(token.expires_at()),
            opt_time(token.redeemed_at()),
        ];
        self.db
            .execute(
                "INSERT INTO mail_tokens \
                 (id, user_id, purpose, digest, payload, expires_at, redeemed_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?) \
                 ON CONFLICT (id) DO UPDATE SET redeemed_at = EXCLUDED.redeemed_at",
                params,
            )
            .await;
    }

    async fn find_by_digest(&self, digest: &str) -> Option<MailToken> {
        load_tokens(
            &self.db,
            format!("SELECT {MAIL_TOKEN_COLUMNS} FROM mail_tokens WHERE digest = ?"),
            vec![Value::Text(digest.to_owned())],
        )
        .await
        .into_iter()
        .next()
    }
}
