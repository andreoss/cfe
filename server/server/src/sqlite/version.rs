use crate::sqlite::conn::Db;
use crate::sqlite::topic::{opt_text, read_time, read_uuid, time_to_value, uuid_value};
use app::VersionRepository;
use domain::{Body, Title, UserId, Version, VersionId, VersionOf};
use rusqlite::Row;
use rusqlite::types::Value;
use time::OffsetDateTime;

pub struct SqliteVersionRepository {
    db: Db,
}

impl SqliteVersionRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

struct VersionRow {
    id: uuid::Uuid,
    subject_kind: String,
    subject_id: uuid::Uuid,
    title: Option<String>,
    body: String,
    editor_id: uuid::Uuid,
    written_at: OffsetDateTime,
}

fn version_row(row: &Row) -> VersionRow {
    VersionRow {
        id: read_uuid(row, 0),
        subject_kind: row.get(1).expect("read kind"),
        subject_id: read_uuid(row, 2),
        title: row.get(3).expect("read title"),
        body: row.get(4).expect("read body"),
        editor_id: read_uuid(row, 5),
        written_at: read_time(row, 6),
    }
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

async fn load_versions(db: &Db, sql: &'static str, params: Vec<Value>) -> Vec<Version> {
    let rows: Vec<VersionRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(sql).expect("prepare versions");
            let mapped = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    Ok(version_row(row))
                })
                .expect("query versions");
            mapped.map(|r| r.expect("read version")).collect()
        })
        .await;
    rows.into_iter().filter_map(to_version).collect()
}

#[async_trait::async_trait]
impl VersionRepository for SqliteVersionRepository {
    async fn save(&self, version: &Version) {
        let params = vec![
            uuid_value(version.id().as_uuid()),
            Value::Text(version.of().as_str().to_owned()),
            uuid_value(version.subject_id()),
            opt_text(version.title().map(|t| t.as_str())),
            Value::Text(version.body().as_str().to_owned()),
            uuid_value(version.editor_id().as_uuid()),
            time_to_value(version.written_at()),
        ];
        self.db
            .execute(
                "INSERT INTO versions (id, subject_kind, subject_id, title, body, editor_id, \
                 written_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                params,
            )
            .await;
    }

    async fn list_for(&self, of: VersionOf, subject_id: uuid::Uuid) -> Vec<Version> {
        let params = vec![Value::Text(of.as_str().to_owned()), uuid_value(subject_id)];
        load_versions(
            &self.db,
            "SELECT id, subject_kind, subject_id, title, body, editor_id, written_at \
             FROM versions WHERE subject_kind = ? AND subject_id = ? ORDER BY written_at ASC",
            params,
        )
        .await
    }

    async fn find(&self, id: VersionId) -> Option<Version> {
        load_versions(
            &self.db,
            "SELECT id, subject_kind, subject_id, title, body, editor_id, written_at \
             FROM versions WHERE id = ?",
            vec![uuid_value(id.as_uuid())],
        )
        .await
        .into_iter()
        .next()
    }
}
