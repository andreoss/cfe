use crate::sqlite::conn::Db;
use crate::sqlite::topic::{count, read_time, read_uuid, time_to_value, uuid_value};
use app::AttachmentRepository;
use domain::{Attachment, AttachmentId, ImageFormat, TopicId, UserId};
use rusqlite::Row;
use rusqlite::types::Value;
use time::OffsetDateTime;

pub struct SqliteAttachmentRepository {
    db: Db,
}

impl SqliteAttachmentRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const ATTACHMENT_COLUMNS: &str = "id, topic_id, content_type, bytes, uploaded_by, uploaded_at";

struct AttachmentRow {
    id: uuid::Uuid,
    topic_id: uuid::Uuid,
    content_type: String,
    bytes: Vec<u8>,
    uploaded_by: uuid::Uuid,
    uploaded_at: OffsetDateTime,
}

fn attachment_row(row: &Row) -> AttachmentRow {
    AttachmentRow {
        id: read_uuid(row, 0),
        topic_id: read_uuid(row, 1),
        content_type: row.get(2).expect("read content type"),
        bytes: row.get(3).expect("read bytes"),
        uploaded_by: read_uuid(row, 4),
        uploaded_at: read_time(row, 5),
    }
}

fn to_attachment(row: AttachmentRow) -> Option<Attachment> {
    let format = ImageFormat::parse(&row.content_type)?;
    Some(Attachment::from_parts(
        AttachmentId::new(row.id),
        TopicId::new(row.topic_id),
        format,
        row.bytes,
        UserId::new(row.uploaded_by),
        row.uploaded_at,
    ))
}

async fn load_attachments(db: &Db, sql: String, params: Vec<Value>) -> Vec<Attachment> {
    let rows: Vec<AttachmentRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare attachments");
            let mapped = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    Ok(attachment_row(row))
                })
                .expect("query attachments");
            mapped.map(|r| r.expect("read attachment")).collect()
        })
        .await;
    rows.into_iter().filter_map(to_attachment).collect()
}

#[async_trait::async_trait]
impl AttachmentRepository for SqliteAttachmentRepository {
    async fn save(&self, attachment: &Attachment) {
        let params = vec![
            uuid_value(attachment.id().as_uuid()),
            uuid_value(attachment.topic_id().as_uuid()),
            Value::Text(attachment.format().content_type().to_owned()),
            Value::Blob(attachment.bytes().to_vec()),
            uuid_value(attachment.uploaded_by().as_uuid()),
            time_to_value(attachment.uploaded_at()),
        ];
        self.db
            .execute(
                "INSERT INTO attachments \
                 (id, topic_id, content_type, bytes, uploaded_by, uploaded_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                params,
            )
            .await;
    }

    async fn find(&self, id: AttachmentId) -> Option<Attachment> {
        load_attachments(
            &self.db,
            format!("SELECT {ATTACHMENT_COLUMNS} FROM attachments WHERE id = ?"),
            vec![uuid_value(id.as_uuid())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn list_for(&self, topic_id: TopicId) -> Vec<Attachment> {
        load_attachments(
            &self.db,
            format!(
                "SELECT {ATTACHMENT_COLUMNS} FROM attachments \
                 WHERE topic_id = ? ORDER BY uploaded_at ASC"
            ),
            vec![uuid_value(topic_id.as_uuid())],
        )
        .await
    }

    async fn count_for(&self, topic_id: TopicId) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM attachments WHERE topic_id = ?",
            vec![uuid_value(topic_id.as_uuid())],
        )
        .await
    }

    async fn delete(&self, id: AttachmentId) {
        self.db
            .execute(
                "DELETE FROM attachments WHERE id = ?",
                vec![uuid_value(id.as_uuid())],
            )
            .await;
    }
}
