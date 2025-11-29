use app::RemarkRepository;
use domain::{Page, Remark, RemarkText, UserId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlRemarkRepository {
    pool: MySqlPool,
}

impl MySqlRemarkRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct RemarkRow {
    author_id: uuid::Uuid,
    subject_id: uuid::Uuid,
    text: String,
    created_at: OffsetDateTime,
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

#[async_trait::async_trait]
impl RemarkRepository for MySqlRemarkRepository {
    async fn save(&self, remark: &Remark) {
        sqlx::query(
            "INSERT INTO remarks (author_id, subject_id, text, created_at) \
             VALUES (?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE text = VALUES(text), created_at = VALUES(created_at)",
        )
        .bind(remark.author_id().as_uuid())
        .bind(remark.subject_id().as_uuid())
        .bind(remark.text().as_str())
        .bind(remark.created_at())
        .execute(&self.pool)
        .await
        .expect("insert remark");
    }

    async fn delete(&self, author_id: UserId, subject_id: UserId) {
        sqlx::query("DELETE FROM remarks WHERE author_id = ? AND subject_id = ?")
            .bind(author_id.as_uuid())
            .bind(subject_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete remark");
    }

    async fn find(&self, author_id: UserId, subject_id: UserId) -> Option<Remark> {
        sqlx::query_as::<_, RemarkRow>(
            "SELECT author_id, subject_id, text, created_at FROM remarks \
             WHERE author_id = ? AND subject_id = ?",
        )
        .bind(author_id.as_uuid())
        .bind(subject_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query remark")
        .and_then(to_remark)
    }

    async fn list_by_author(&self, author_id: UserId, page: Page) -> Vec<Remark> {
        sqlx::query_as::<_, RemarkRow>(
            "SELECT author_id, subject_id, text, created_at FROM remarks \
             WHERE author_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(author_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query remarks")
        .into_iter()
        .filter_map(to_remark)
        .collect()
    }

    async fn count_by_author(&self, author_id: UserId) -> u64 {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM remarks WHERE author_id = ?")
                .bind(author_id.as_uuid())
                .fetch_one(&self.pool)
                .await
                .expect("count remarks");
        count as u64
    }
}
