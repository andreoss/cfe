use app::ReactionRepository;
use domain::{Reaction, ReactionKind, ReactionTarget, UserId};
use sqlx::{FromRow, MySqlPool};

pub struct MySqlReactionRepository {
    pool: MySqlPool,
}

impl MySqlReactionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct CountRow {
    kind: String,
    total: i64,
}

#[async_trait::async_trait]
impl ReactionRepository for MySqlReactionRepository {
    async fn save(&self, reaction: &Reaction) {
        sqlx::query(
            "INSERT INTO reactions (user_id, target_kind, target_id, kind, created_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE kind = VALUES(kind), created_at = VALUES(created_at)",
        )
        .bind(reaction.user_id().as_uuid())
        .bind(reaction.target().kind_str())
        .bind(reaction.target().id())
        .bind(reaction.kind().as_str())
        .bind(reaction.created_at())
        .execute(&self.pool)
        .await
        .expect("insert reaction");
    }

    async fn delete(&self, user_id: UserId, target: ReactionTarget) {
        sqlx::query(
            "DELETE FROM reactions WHERE user_id = ? AND target_kind = ? AND target_id = ?",
        )
        .bind(user_id.as_uuid())
        .bind(target.kind_str())
        .bind(target.id())
        .execute(&self.pool)
        .await
        .expect("delete reaction");
    }

    async fn counts(&self, target: ReactionTarget) -> Vec<(ReactionKind, u64)> {
        sqlx::query_as::<_, CountRow>(
            "SELECT kind, COUNT(*) AS total FROM reactions \
             WHERE target_kind = ? AND target_id = ? GROUP BY kind",
        )
        .bind(target.kind_str())
        .bind(target.id())
        .fetch_all(&self.pool)
        .await
        .expect("count reactions")
        .into_iter()
        .filter_map(|row| {
            ReactionKind::parse(&row.kind)
                .ok()
                .map(|kind| (kind, row.total as u64))
        })
        .collect()
    }

    async fn find_mine(&self, user_id: UserId, target: ReactionTarget) -> Option<ReactionKind> {
        let kind: Option<String> = sqlx::query_scalar(
            "SELECT kind FROM reactions \
             WHERE user_id = ? AND target_kind = ? AND target_id = ?",
        )
        .bind(user_id.as_uuid())
        .bind(target.kind_str())
        .bind(target.id())
        .fetch_optional(&self.pool)
        .await
        .expect("query own reaction");
        kind.and_then(|k| ReactionKind::parse(&k).ok())
    }
}
