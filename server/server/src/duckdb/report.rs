use crate::duckdb::conn::Db;
use crate::duckdb::topic::{
    count, limit_value, offset_value, opt_time, opt_uuid, read_opt_time, read_opt_uuid, read_time,
    read_uuid, time_to_value, uuid_value,
};
use app::ReportRepository;
use domain::{
    CommentId, Page, Reason, Report, ReportId, ReportKind, ReportTarget, TopicId, UserId,
};
use duckdb::Row;
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckReportRepository {
    db: Db,
}

impl DuckReportRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const REPORT_COLUMNS: &str = "id, topic_id, comment_id, reporter_id, kind, reason, created_at, \
     closed_by, closed_at";

struct ReportRow {
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

fn report_row(row: &Row) -> ReportRow {
    ReportRow {
        id: read_uuid(row, 0),
        topic_id: read_uuid(row, 1),
        comment_id: read_opt_uuid(row, 2),
        reporter_id: read_uuid(row, 3),
        kind: row.get(4).expect("read kind"),
        reason: row.get(5).expect("read reason"),
        created_at: read_time(row, 6),
        closed_by: read_opt_uuid(row, 7),
        closed_at: read_opt_time(row, 8),
    }
}

fn to_report(row: ReportRow) -> Report {
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

async fn load_reports(db: &Db, sql: String, params: Vec<Value>) -> Vec<Report> {
    let rows: Vec<ReportRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare reports");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(report_row(row))
                })
                .expect("query reports");
            mapped.map(|r| r.expect("read report")).collect()
        })
        .await;
    rows.into_iter().map(to_report).collect()
}

#[async_trait::async_trait]
impl ReportRepository for DuckReportRepository {
    async fn save(&self, report: &Report) {
        let params = vec![
            uuid_value(report.id().as_uuid()),
            uuid_value(report.target().topic_id().as_uuid()),
            opt_uuid(report.target().comment_id().map(|c| c.as_uuid())),
            uuid_value(report.reporter_id().as_uuid()),
            Value::Text(report.kind().as_str().to_owned()),
            Value::Text(report.reason().as_str().to_owned()),
            time_to_value(report.created_at()),
        ];
        self.db
            .execute(
                "INSERT INTO reports \
                 (id, topic_id, comment_id, reporter_id, kind, reason, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params,
            )
            .await;
    }

    async fn update(&self, report: &Report) {
        let params = vec![
            opt_uuid(report.closed_by().map(|u| u.as_uuid())),
            opt_time(report.closed_at()),
            uuid_value(report.id().as_uuid()),
        ];
        self.db
            .execute(
                "UPDATE reports SET closed_by = ?, closed_at = ? WHERE id = ?",
                params,
            )
            .await;
    }

    async fn find_by_id(&self, id: ReportId) -> Option<Report> {
        load_reports(
            &self.db,
            format!("SELECT {REPORT_COLUMNS} FROM reports WHERE id = ?"),
            vec![uuid_value(id.as_uuid())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn list_open(&self, page: Page) -> Vec<Report> {
        load_reports(
            &self.db,
            format!(
                "SELECT {REPORT_COLUMNS} FROM reports \
                 WHERE closed_at IS NULL ORDER BY created_at DESC LIMIT ? OFFSET ?"
            ),
            vec![limit_value(page), offset_value(page)],
        )
        .await
    }

    async fn count_open(&self) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM reports WHERE closed_at IS NULL",
            Vec::new(),
        )
        .await
    }

    async fn count_open_for_topic(&self, topic_id: TopicId) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM reports WHERE topic_id = ? AND closed_at IS NULL",
            vec![uuid_value(topic_id.as_uuid())],
        )
        .await
    }

    async fn find_open_by_reporter(
        &self,
        reporter_id: UserId,
        target: ReportTarget,
    ) -> Option<Report> {
        let (clause, params) = match target.comment_id() {
            Some(comment_id) => (
                "reporter_id = ? AND comment_id = ? AND closed_at IS NULL",
                vec![
                    uuid_value(reporter_id.as_uuid()),
                    uuid_value(comment_id.as_uuid()),
                ],
            ),
            None => (
                "reporter_id = ? AND topic_id = ? AND comment_id IS NULL AND closed_at IS NULL",
                vec![
                    uuid_value(reporter_id.as_uuid()),
                    uuid_value(target.topic_id().as_uuid()),
                ],
            ),
        };
        load_reports(
            &self.db,
            format!("SELECT {REPORT_COLUMNS} FROM reports WHERE {clause}"),
            params,
        )
        .await
        .into_iter()
        .next()
    }

    async fn count_by_reporter_since(&self, reporter_id: UserId, since: OffsetDateTime) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM reports WHERE reporter_id = ? AND created_at >= ?",
            vec![uuid_value(reporter_id.as_uuid()), time_to_value(since)],
        )
        .await
    }
}
