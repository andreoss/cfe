use crate::sqlite::conn::Db;
use crate::sqlite::topic::{time_to_value, uuid_value};
use app::ReactionRepository;
use domain::{Reaction, ReactionKind, ReactionTarget, UserId};
use rusqlite::types::Value;

pub struct SqliteReactionRepository {
    db: Db,
}

impl SqliteReactionRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl ReactionRepository for SqliteReactionRepository {
    async fn save(&self, reaction: &Reaction) {
        let params = vec![
            uuid_value(reaction.user_id().as_uuid()),
            Value::Text(reaction.target().kind_str().to_owned()),
            uuid_value(reaction.target().id()),
            Value::Text(reaction.kind().as_str().to_owned()),
            time_to_value(reaction.created_at()),
        ];
        self.db
            .execute(
                "INSERT INTO reactions (user_id, target_kind, target_id, kind, created_at) \
                 VALUES (?, ?, ?, ?, ?) \
                 ON CONFLICT (user_id, target_kind, target_id) \
                 DO UPDATE SET kind = EXCLUDED.kind, created_at = EXCLUDED.created_at",
                params,
            )
            .await;
    }

    async fn delete(&self, user_id: UserId, target: ReactionTarget) {
        let params = vec![
            uuid_value(user_id.as_uuid()),
            Value::Text(target.kind_str().to_owned()),
            uuid_value(target.id()),
        ];
        self.db
            .execute(
                "DELETE FROM reactions WHERE user_id = ? AND target_kind = ? AND target_id = ?",
                params,
            )
            .await;
    }

    async fn counts(&self, target: ReactionTarget) -> Vec<(ReactionKind, u64)> {
        let params = vec![
            Value::Text(target.kind_str().to_owned()),
            uuid_value(target.id()),
        ];
        let rows: Vec<(String, i64)> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT kind, COUNT(*) AS total FROM reactions \
                         WHERE target_kind = ? AND target_id = ? GROUP BY kind",
                    )
                    .expect("prepare counts");
                let mapped = stmt
                    .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                        Ok((
                            row.get::<_, String>(0).expect("read kind"),
                            row.get::<_, i64>(1).expect("read total"),
                        ))
                    })
                    .expect("count reactions");
                mapped.map(|r| r.expect("read count")).collect()
            })
            .await;
        rows.into_iter()
            .filter_map(|(kind, total)| {
                ReactionKind::parse(&kind)
                    .ok()
                    .map(|kind| (kind, total as u64))
            })
            .collect()
    }

    async fn find_mine(&self, user_id: UserId, target: ReactionTarget) -> Option<ReactionKind> {
        let params = vec![
            uuid_value(user_id.as_uuid()),
            Value::Text(target.kind_str().to_owned()),
            uuid_value(target.id()),
        ];
        let kinds: Vec<String> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT kind FROM reactions \
                         WHERE user_id = ? AND target_kind = ? AND target_id = ?",
                    )
                    .expect("prepare own reaction");
                let mapped = stmt
                    .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                        row.get::<_, String>(0)
                    })
                    .expect("query own reaction");
                mapped.map(|r| r.expect("read kind")).collect()
            })
            .await;
        kinds
            .into_iter()
            .next()
            .and_then(|kind| ReactionKind::parse(&kind).ok())
    }
}
