use crate::sqlite::conn::Db;
use crate::sqlite::topic::{
    bool_value, opt_time, opt_uuid, read_opt_time, read_opt_uuid, read_time, read_uuid,
    time_to_value, uuid_value,
};
use app::EnforcementRepository;
use domain::{Ban, Reason, UserId, Warning, WarningId};
use rusqlite::types::Value;
use time::OffsetDateTime;

pub struct SqliteEnforcementRepository {
    db: Db,
}

impl SqliteEnforcementRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

struct BanRow {
    moderator_id: Option<uuid::Uuid>,
    reason: String,
    banned_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
}

fn stored_moderator(id: UserId) -> Value {
    if id.is_scheduled_work() {
        opt_uuid(None)
    } else {
        opt_uuid(Some(id.as_uuid()))
    }
}

struct WarningRow {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    moderator_id: uuid::Uuid,
    reason: String,
    created_at: OffsetDateTime,
    acknowledged: bool,
}

#[async_trait::async_trait]
impl EnforcementRepository for SqliteEnforcementRepository {
    async fn save_ban(&self, user_id: UserId, ban: &Ban) {
        let params = vec![
            uuid_value(user_id.as_uuid()),
            stored_moderator(ban.moderator_id()),
            Value::Text(ban.reason().as_str().to_owned()),
            time_to_value(ban.banned_at()),
            opt_time(ban.until()),
        ];
        self.db
            .execute(
                "INSERT INTO bans (user_id, moderator_id, reason, banned_at, until) \
                 VALUES (?, ?, ?, ?, ?) \
                 ON CONFLICT (user_id) DO UPDATE SET moderator_id = EXCLUDED.moderator_id, \
                 reason = EXCLUDED.reason, banned_at = EXCLUDED.banned_at, until = EXCLUDED.until",
                params,
            )
            .await;
    }

    async fn find_ban(&self, user_id: UserId) -> Option<Ban> {
        let id = user_id.as_uuid();
        let rows: Vec<BanRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT moderator_id, reason, banned_at, until FROM bans WHERE user_id = ?",
                    )
                    .expect("prepare ban");
                let mapped = stmt
                    .query_map([uuid_value(id)], |row| {
                        Ok(BanRow {
                            moderator_id: read_opt_uuid(row, 0),
                            reason: row.get(1).expect("read reason"),
                            banned_at: read_time(row, 2),
                            until: read_opt_time(row, 3),
                        })
                    })
                    .expect("query ban");
                mapped.map(|r| r.expect("read ban")).collect()
            })
            .await;
        let row = rows.into_iter().next()?;
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
        self.db
            .execute(
                "DELETE FROM bans WHERE user_id = ?",
                vec![uuid_value(user_id.as_uuid())],
            )
            .await;
    }

    async fn save_warning(&self, warning: &Warning) {
        let params = vec![
            uuid_value(warning.id().as_uuid()),
            uuid_value(warning.user_id().as_uuid()),
            uuid_value(warning.moderator_id().as_uuid()),
            Value::Text(warning.reason().as_str().to_owned()),
            time_to_value(warning.created_at()),
            bool_value(warning.is_acknowledged()),
        ];
        self.db
            .execute(
                "INSERT INTO warnings (id, user_id, moderator_id, reason, created_at, \
                 acknowledged) VALUES (?, ?, ?, ?, ?, ?) \
                 ON CONFLICT (id) DO UPDATE SET acknowledged = EXCLUDED.acknowledged",
                params,
            )
            .await;
    }

    async fn list_warnings(&self, user_id: UserId) -> Vec<Warning> {
        let id = user_id.as_uuid();
        let rows: Vec<WarningRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, user_id, moderator_id, reason, created_at, acknowledged \
                         FROM warnings WHERE user_id = ? ORDER BY created_at DESC",
                    )
                    .expect("prepare warnings");
                let mapped = stmt
                    .query_map([uuid_value(id)], |row| {
                        Ok(WarningRow {
                            id: read_uuid(row, 0),
                            user_id: read_uuid(row, 1),
                            moderator_id: read_uuid(row, 2),
                            reason: row.get(3).expect("read reason"),
                            created_at: read_time(row, 4),
                            acknowledged: row.get(5).expect("read acknowledged"),
                        })
                    })
                    .expect("query warnings");
                mapped.map(|r| r.expect("read warning")).collect()
            })
            .await;
        rows.into_iter()
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
        let params = vec![
            uuid_value(user_id.as_uuid()),
            uuid_value(ignored_id.as_uuid()),
        ];
        self.db
            .execute(
                "INSERT INTO ignores (user_id, ignored_id) VALUES (?, ?) \
                 ON CONFLICT (user_id, ignored_id) DO NOTHING",
                params,
            )
            .await;
    }

    async fn delete_ignore(&self, user_id: UserId, ignored_id: UserId) {
        let params = vec![
            uuid_value(user_id.as_uuid()),
            uuid_value(ignored_id.as_uuid()),
        ];
        self.db
            .execute(
                "DELETE FROM ignores WHERE user_id = ? AND ignored_id = ?",
                params,
            )
            .await;
    }

    async fn list_ignored(&self, user_id: UserId) -> Vec<UserId> {
        let id = user_id.as_uuid();
        let rows: Vec<uuid::Uuid> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT ignored_id FROM ignores WHERE user_id = ?")
                    .expect("prepare ignores");
                let mapped = stmt
                    .query_map([uuid_value(id)], |row| Ok(read_uuid(row, 0)))
                    .expect("query ignores");
                mapped.map(|r| r.expect("read ignore")).collect()
            })
            .await;
        rows.into_iter().map(UserId::new).collect()
    }
}
