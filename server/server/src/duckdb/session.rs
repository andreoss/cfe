use crate::duckdb::conn::Db;
use crate::duckdb::topic::{read_time, read_uuid, time_to_value, uuid_value};
use app::SessionRepository;
use domain::{Session, SessionId, SessionToken, UserId};
use duckdb::Row;
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckSessionRepository {
    db: Db,
}

impl DuckSessionRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const SESSION_COLUMNS: &str = "id, user_id, token, expires_at";

struct SessionRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    token: String,
    expires_at: OffsetDateTime,
}

fn session_row(row: &Row) -> SessionRow {
    SessionRow {
        id: read_uuid(row, 0),
        user_id: read_uuid(row, 1),
        token: row.get(2).expect("read token"),
        expires_at: read_time(row, 3),
    }
}

fn to_session(row: SessionRow) -> Session {
    Session::new(
        SessionId::new(row.id),
        UserId::new(row.user_id),
        SessionToken::parse(&row.token).expect("stored token is valid"),
        row.expires_at,
    )
}

async fn load_sessions(db: &Db, sql: String, params: Vec<Value>) -> Vec<Session> {
    let rows: Vec<SessionRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare sessions");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(session_row(row))
                })
                .expect("query sessions");
            mapped.map(|r| r.expect("read session")).collect()
        })
        .await;
    rows.into_iter().map(to_session).collect()
}

#[async_trait::async_trait]
impl SessionRepository for DuckSessionRepository {
    async fn save(&self, session: &Session) {
        let params = vec![
            uuid_value(session.id().as_uuid()),
            uuid_value(session.user_id().as_uuid()),
            Value::Text(session.token().as_str().to_owned()),
            time_to_value(session.expires_at()),
        ];
        self.db
            .execute(
                "INSERT INTO sessions (id, user_id, token, expires_at) VALUES (?, ?, ?, ?)",
                params,
            )
            .await;
    }

    async fn find_by_token(&self, token: &SessionToken) -> Option<Session> {
        load_sessions(
            &self.db,
            format!("SELECT {SESSION_COLUMNS} FROM sessions WHERE token = ?"),
            vec![Value::Text(token.as_str().to_owned())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn touch(&self, id: SessionId, new_expiry: OffsetDateTime) {
        self.db
            .execute(
                "UPDATE sessions SET expires_at = ? WHERE id = ?",
                vec![time_to_value(new_expiry), uuid_value(id.as_uuid())],
            )
            .await;
    }

    async fn delete(&self, id: SessionId) {
        self.db
            .execute(
                "DELETE FROM sessions WHERE id = ?",
                vec![uuid_value(id.as_uuid())],
            )
            .await;
    }

    async fn delete_for_user(&self, user_id: UserId) {
        self.db
            .execute(
                "DELETE FROM sessions WHERE user_id = ?",
                vec![uuid_value(user_id.as_uuid())],
            )
            .await;
    }
}
