use crate::sqlite::conn::Db;
use crate::sqlite::topic::{
    count, limit_value, offset_value, read_time, read_uuid, time_to_value, uuid_value,
};
use app::RemarkRepository;
use domain::{Page, Remark, RemarkText, UserId};
use rusqlite::Row;
use rusqlite::types::Value;
use time::OffsetDateTime;

pub struct SqliteRemarkRepository {
    db: Db,
}

impl SqliteRemarkRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

struct RemarkRow {
    author_id: uuid::Uuid,
    subject_id: uuid::Uuid,
    text: String,
    created_at: OffsetDateTime,
}

fn remark_row(row: &Row) -> RemarkRow {
    RemarkRow {
        author_id: read_uuid(row, 0),
        subject_id: read_uuid(row, 1),
        text: row.get(2).expect("read note"),
        created_at: read_time(row, 3),
    }
}

fn to_remark(row: RemarkRow) -> Option<Remark> {
    RemarkText::parse(&row.text).ok().map(|text| {
        Remark::new(
            UserId::new(row.author_id),
            UserId::new(row.subject_id),
            text,
            row.created_at,
        )
    })
}

async fn load_remarks(db: &Db, sql: &'static str, params: Vec<Value>) -> Vec<Remark> {
    let rows: Vec<RemarkRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(sql).expect("prepare remarks");
            let mapped = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    Ok(remark_row(row))
                })
                .expect("query remarks");
            mapped.map(|r| r.expect("read remark")).collect()
        })
        .await;
    rows.into_iter().filter_map(to_remark).collect()
}

#[async_trait::async_trait]
impl RemarkRepository for SqliteRemarkRepository {
    async fn save(&self, remark: &Remark) {
        let params = vec![
            uuid_value(remark.author_id().as_uuid()),
            uuid_value(remark.subject_id().as_uuid()),
            Value::Text(remark.text().as_str().to_owned()),
            time_to_value(remark.created_at()),
        ];
        self.db
            .execute(
                "INSERT INTO remarks (author_id, subject_id, text, created_at) \
                 VALUES (?, ?, ?, ?) \
                 ON CONFLICT (author_id, subject_id) \
                 DO UPDATE SET text = excluded.text, created_at = excluded.created_at",
                params,
            )
            .await;
    }

    async fn delete(&self, author_id: UserId, subject_id: UserId) {
        self.db
            .execute(
                "DELETE FROM remarks WHERE author_id = ? AND subject_id = ?",
                vec![
                    uuid_value(author_id.as_uuid()),
                    uuid_value(subject_id.as_uuid()),
                ],
            )
            .await;
    }

    async fn find(&self, author_id: UserId, subject_id: UserId) -> Option<Remark> {
        let params = vec![
            uuid_value(author_id.as_uuid()),
            uuid_value(subject_id.as_uuid()),
        ];
        load_remarks(
            &self.db,
            "SELECT author_id, subject_id, text, created_at FROM remarks \
             WHERE author_id = ? AND subject_id = ?",
            params,
        )
        .await
        .into_iter()
        .next()
    }

    async fn list_by_author(&self, author_id: UserId, page: Page) -> Vec<Remark> {
        let params = vec![
            uuid_value(author_id.as_uuid()),
            limit_value(page),
            offset_value(page),
        ];
        load_remarks(
            &self.db,
            "SELECT author_id, subject_id, text, created_at FROM remarks \
             WHERE author_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
            params,
        )
        .await
    }

    async fn count_by_author(&self, author_id: UserId) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM remarks WHERE author_id = ?",
            vec![uuid_value(author_id.as_uuid())],
        )
        .await
    }
}
