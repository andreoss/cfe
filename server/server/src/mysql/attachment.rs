use app::AttachmentRepository;
use domain::{Attachment, AttachmentId, ImageFormat, TopicId, UserId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlAttachmentRepository {
    pool: MySqlPool,
}

impl MySqlAttachmentRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

const ATTACHMENT_COLUMNS: &str = "id, topic_id, content_type, bytes, uploaded_by, uploaded_at";

#[derive(FromRow)]
struct AttachmentRow {
    id: uuid::Uuid,
    topic_id: uuid::Uuid,
    content_type: String,
    bytes: Vec<u8>,
    uploaded_by: uuid::Uuid,
    uploaded_at: OffsetDateTime,
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

#[async_trait::async_trait]
impl AttachmentRepository for MySqlAttachmentRepository {
    async fn save(&self, attachment: &Attachment) {
        sqlx::query(
            "INSERT INTO attachments (id, topic_id, content_type, bytes, uploaded_by, uploaded_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(attachment.id().as_uuid())
        .bind(attachment.topic_id().as_uuid())
        .bind(attachment.format().content_type())
        .bind(attachment.bytes().to_vec())
        .bind(attachment.uploaded_by().as_uuid())
        .bind(attachment.uploaded_at())
        .execute(&self.pool)
        .await
        .expect("insert attachment");
    }

    async fn find(&self, id: AttachmentId) -> Option<Attachment> {
        sqlx::query_as::<_, AttachmentRow>(&format!(
            "SELECT {ATTACHMENT_COLUMNS} FROM attachments WHERE id = ?"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query attachment")
        .and_then(to_attachment)
    }

    async fn list_for(&self, topic_id: TopicId) -> Vec<Attachment> {
        sqlx::query_as::<_, AttachmentRow>(&format!(
            "SELECT {ATTACHMENT_COLUMNS} FROM attachments \
             WHERE topic_id = ? ORDER BY uploaded_at ASC"
        ))
        .bind(topic_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query attachments")
        .into_iter()
        .filter_map(to_attachment)
        .collect()
    }

    async fn count_for(&self, topic_id: TopicId) -> u64 {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM attachments WHERE topic_id = ?")
            .bind(topic_id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .expect("count attachments");
        count as u64
    }

    async fn delete(&self, id: AttachmentId) {
        sqlx::query("DELETE FROM attachments WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete attachment");
    }
}
