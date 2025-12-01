use app::VersionRepository;
use domain::{Body, Title, UserId, Version, VersionId, VersionOf};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgVersionRepository {
    pool: PgPool,
}

impl PgVersionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct VersionRow {
    id: uuid::Uuid,
    subject_kind: String,
    subject_id: uuid::Uuid,
    title: Option<String>,
    body: String,
    editor_id: uuid::Uuid,
    written_at: OffsetDateTime,
}

fn to_version(row: VersionRow) -> Option<Version> {
    let of = VersionOf::parse(&row.subject_kind).ok()?;
    let title = match &row.title {
        Some(title) => Some(Title::parse(title).ok()?),
        None => None,
    };
    let body = Body::parse(&row.body).ok()?;
    Some(Version::new(
        VersionId::new(row.id),
        of,
        row.subject_id,
        title,
        body,
        UserId::new(row.editor_id),
        row.written_at,
    ))
}

#[async_trait::async_trait]
impl VersionRepository for PgVersionRepository {
    async fn save(&self, version: &Version) {
        sqlx::query(
            "INSERT INTO versions (id, subject_kind, subject_id, title, body, editor_id, \
             written_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(version.id().as_uuid())
        .bind(version.of().as_str())
        .bind(version.subject_id())
        .bind(version.title().map(|t| t.as_str()))
        .bind(version.body().as_str())
        .bind(version.editor_id().as_uuid())
        .bind(version.written_at())
        .execute(&self.pool)
        .await
        .expect("insert version");
    }

    async fn list_for(&self, of: VersionOf, subject_id: uuid::Uuid) -> Vec<Version> {
        sqlx::query_as::<_, VersionRow>(
            "SELECT id, subject_kind, subject_id, title, body, editor_id, written_at \
             FROM versions WHERE subject_kind = $1 AND subject_id = $2 ORDER BY written_at ASC",
        )
        .bind(of.as_str())
        .bind(subject_id)
        .fetch_all(&self.pool)
        .await
        .expect("query versions")
        .into_iter()
        .filter_map(to_version)
        .collect()
    }

    async fn find(&self, id: VersionId) -> Option<Version> {
        sqlx::query_as::<_, VersionRow>(
            "SELECT id, subject_kind, subject_id, title, body, editor_id, written_at \
             FROM versions WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query version")
        .and_then(to_version)
    }
}
