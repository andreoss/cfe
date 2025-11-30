use crate::duckdb::conn::Db;
use crate::duckdb::topic::{
    count, limit_value, offset_value, opt_time, opt_uuid, read_opt_time, read_opt_uuid, read_time,
    read_uuid, time_to_value, uuid_value,
};
use app::InvitationRepository;
use domain::{Invitation, InvitationCode, InvitationId, Page, UserId};
use duckdb::Row;
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckInvitationRepository {
    db: Db,
}

impl DuckInvitationRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const INVITATION_COLUMNS: &str = "id, code, issuer_id, created_at, expires_at, spent_at, spent_by";

struct InvitationRow {
    id: uuid::Uuid,
    code: String,
    issuer_id: uuid::Uuid,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
    spent_at: Option<OffsetDateTime>,
    spent_by: Option<uuid::Uuid>,
}

fn invitation_row(row: &Row) -> InvitationRow {
    InvitationRow {
        id: read_uuid(row, 0),
        code: row.get(1).expect("read code"),
        issuer_id: read_uuid(row, 2),
        created_at: read_time(row, 3),
        expires_at: read_time(row, 4),
        spent_at: read_opt_time(row, 5),
        spent_by: read_opt_uuid(row, 6),
    }
}

fn to_invitation(row: InvitationRow) -> Option<Invitation> {
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

async fn load_invitations(db: &Db, sql: String, params: Vec<Value>) -> Vec<Invitation> {
    let rows: Vec<InvitationRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare invitations");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(invitation_row(row))
                })
                .expect("query invitations");
            mapped.map(|r| r.expect("read invitation")).collect()
        })
        .await;
    rows.into_iter().filter_map(to_invitation).collect()
}

#[async_trait::async_trait]
impl InvitationRepository for DuckInvitationRepository {
    async fn save(&self, invitation: &Invitation) {
        let params = vec![
            uuid_value(invitation.id().as_uuid()),
            Value::Text(invitation.code().as_str().to_owned()),
            uuid_value(invitation.issuer_id().as_uuid()),
            time_to_value(invitation.created_at()),
            time_to_value(invitation.expires_at()),
            opt_time(invitation.spent_at()),
            opt_uuid(invitation.spent_by().map(|by| by.as_uuid())),
        ];
        self.db
            .execute(
                "INSERT INTO invitations \
                 (id, code, issuer_id, created_at, expires_at, spent_at, spent_by) \
                 VALUES (?, ?, ?, ?, ?, ?, ?) \
                 ON CONFLICT (id) DO UPDATE SET spent_at = EXCLUDED.spent_at, \
                 spent_by = EXCLUDED.spent_by",
                params,
            )
            .await;
    }

    async fn claim(&self, id: InvitationId, at: OffsetDateTime) -> bool {
        let params = [time_to_value(at), uuid_value(id.as_uuid())];
        let changed: usize = self
            .db
            .call(move |conn| {
                conn.execute(
                    "UPDATE invitations SET spent_at = ? WHERE id = ? AND spent_at IS NULL",
                    duckdb::params_from_iter(params.iter()),
                )
                .expect("claim invitation")
            })
            .await;
        changed == 1
    }

    async fn attribute(&self, id: InvitationId, by: UserId) {
        self.db
            .execute(
                "UPDATE invitations SET spent_by = ? WHERE id = ?",
                vec![uuid_value(by.as_uuid()), uuid_value(id.as_uuid())],
            )
            .await;
    }

    async fn release(&self, id: InvitationId) {
        self.db
            .execute(
                "UPDATE invitations SET spent_at = NULL, spent_by = NULL WHERE id = ?",
                vec![uuid_value(id.as_uuid())],
            )
            .await;
    }

    async fn find_by_code(&self, code: &InvitationCode) -> Option<Invitation> {
        load_invitations(
            &self.db,
            format!("SELECT {INVITATION_COLUMNS} FROM invitations WHERE code = ?"),
            vec![Value::Text(code.as_str().to_owned())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn list_by_issuer(&self, issuer_id: UserId, page: Page) -> Vec<Invitation> {
        load_invitations(
            &self.db,
            format!(
                "SELECT {INVITATION_COLUMNS} FROM invitations WHERE issuer_id = ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            ),
            vec![
                uuid_value(issuer_id.as_uuid()),
                limit_value(page),
                offset_value(page),
            ],
        )
        .await
    }

    async fn count_by_issuer(&self, issuer_id: UserId) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM invitations WHERE issuer_id = ?",
            vec![uuid_value(issuer_id.as_uuid())],
        )
        .await
    }

    async fn count_outstanding(&self, issuer_id: UserId, now: OffsetDateTime) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM invitations \
             WHERE issuer_id = ? AND spent_at IS NULL AND expires_at > ?",
            vec![uuid_value(issuer_id.as_uuid()), time_to_value(now)],
        )
        .await
    }
}
