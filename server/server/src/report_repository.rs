use app::ReportRepository;
use domain::{
    CommentId, Page, Reason, Report, ReportId, ReportKind, ReportTarget, TopicId, UserId,
};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgReportRepository {
    pool: PgPool,
}

impl PgReportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, topic_id, comment_id, reporter_id, kind, reason, created_at, \
     closed_by, closed_at";

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    topic_id: uuid::Uuid,
    comment_id: Option<uuid::Uuid>,
    reporter_id: uuid::Uuid,
    kind: String,
    reason: String,
    created_at: OffsetDateTime,
    closed_by: Option<uuid::Uuid>,
    closed_at: Option<OffsetDateTime>,
}

fn to_report(row: Row) -> Report {
    let topic_id = TopicId::new(row.topic_id);
    let target = match row.comment_id {
        Some(comment_id) => ReportTarget::Comment(topic_id, CommentId::new(comment_id)),
        None => ReportTarget::Topic(topic_id),
    };
    Report::from_parts(
        ReportId::new(row.id),
        target,
        UserId::new(row.reporter_id),
        ReportKind::parse(&row.kind).expect("stored kind is valid"),
        Reason::parse(&row.reason).expect("stored reason is valid"),
        row.created_at,
        row.closed_by.map(UserId::new),
        row.closed_at,
    )
}

#[async_trait::async_trait]
impl ReportRepository for PgReportRepository {
    async fn save(&self, report: &Report) {
        sqlx::query(
            "INSERT INTO reports \
             (id, topic_id, comment_id, reporter_id, kind, reason, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(report.id().as_uuid())
        .bind(report.target().topic_id().as_uuid())
        .bind(report.target().comment_id().map(|c| c.as_uuid()))
        .bind(report.reporter_id().as_uuid())
        .bind(report.kind().as_str())
        .bind(report.reason().as_str())
        .bind(report.created_at())
        .execute(&self.pool)
        .await
        .expect("insert report");
    }

    async fn update(&self, report: &Report) {
        sqlx::query("UPDATE reports SET closed_by = $2, closed_at = $3 WHERE id = $1")
            .bind(report.id().as_uuid())
            .bind(report.closed_by().map(|u| u.as_uuid()))
            .bind(report.closed_at())
            .execute(&self.pool)
            .await
            .expect("update report");
    }

    async fn find_by_id(&self, id: ReportId) -> Option<Report> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM reports WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query report")
        .map(to_report)
    }

    async fn list_open(&self, page: Page) -> Vec<Report> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM reports \
             WHERE closed_at IS NULL ORDER BY created_at DESC \
             LIMIT $1 OFFSET $2"
        ))
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query reports")
        .into_iter()
        .map(to_report)
        .collect()
    }

    async fn count_open(&self) -> u64 {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reports WHERE closed_at IS NULL")
            .fetch_one(&self.pool)
            .await
            .expect("count reports");
        count as u64
    }

    async fn count_open_for_topic(&self, topic_id: TopicId) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reports WHERE topic_id = $1 AND closed_at IS NULL",
        )
        .bind(topic_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count reports for topic");
        count as u64
    }

    async fn find_open_by_reporter(
        &self,
        reporter_id: UserId,
        target: ReportTarget,
    ) -> Option<Report> {
        let (clause, scope) = match target.comment_id() {
            Some(comment_id) => ("comment_id = $2", comment_id.as_uuid()),
            None => (
                "topic_id = $2 AND comment_id IS NULL",
                target.topic_id().as_uuid(),
            ),
        };
        let sql = format!(
            "SELECT {SELECT_COLUMNS} FROM reports \
             WHERE reporter_id = $1 AND {clause} AND closed_at IS NULL"
        );
        sqlx::query_as::<_, Row>(&sql)
            .bind(reporter_id.as_uuid())
            .bind(scope)
            .fetch_optional(&self.pool)
            .await
            .expect("query open report")
            .map(to_report)
    }

    async fn count_by_reporter_since(&self, reporter_id: UserId, since: OffsetDateTime) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reports WHERE reporter_id = $1 AND created_at >= $2",
        )
        .bind(reporter_id.as_uuid())
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .expect("count reports by reporter");
        count as u64
    }
}
