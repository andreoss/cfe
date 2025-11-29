use app::RemarkRepository;
use domain::{Page, Remark, RemarkText, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgRemarkRepository {
    pool: PgPool,
}

impl PgRemarkRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct Row {
    author_id: uuid::Uuid,
    subject_id: uuid::Uuid,
    text: String,
    created_at: OffsetDateTime,
}

fn to_remark(row: Row) -> Option<Remark> {
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
impl RemarkRepository for PgRemarkRepository {
    async fn save(&self, remark: &Remark) {
        sqlx::query(
            "INSERT INTO remarks (author_id, subject_id, text, created_at) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (author_id, subject_id) \
             DO UPDATE SET text = EXCLUDED.text, created_at = EXCLUDED.created_at",
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
        sqlx::query("DELETE FROM remarks WHERE author_id = $1 AND subject_id = $2")
            .bind(author_id.as_uuid())
            .bind(subject_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete remark");
    }

    async fn find(&self, author_id: UserId, subject_id: UserId) -> Option<Remark> {
        sqlx::query_as::<_, Row>(
            "SELECT author_id, subject_id, text, created_at FROM remarks \
             WHERE author_id = $1 AND subject_id = $2",
        )
        .bind(author_id.as_uuid())
        .bind(subject_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query remark")
        .and_then(to_remark)
    }

    async fn list_by_author(&self, author_id: UserId, page: Page) -> Vec<Remark> {
        sqlx::query_as::<_, Row>(
            "SELECT author_id, subject_id, text, created_at FROM remarks \
             WHERE author_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
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
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM remarks WHERE author_id = $1")
            .bind(author_id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .expect("count remarks");
        count as u64
    }
}
