use crate::duckdb::conn::Db;
use crate::duckdb::topic::{
    count, limit_value, offset_value, opt_text, opt_time, opt_uuid, penalty_value, read_opt_time,
    read_opt_uuid, read_time, read_uuid, revision, time_to_value, uuid_value,
};
use app::CommentRepository;
use domain::{Body, Comment, CommentId, Deletion, Page, Penalty, Reason, TopicId, UserId};
use duckdb::Row;
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckCommentRepository {
    db: Db,
}

impl DuckCommentRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

pub const COMMENT_COLUMNS: &str = "id, topic_id, author_id, parent_id, body, created_at, \
    deleted_reason, deleted_by, deleted_at, deletion_penalty, edited_by, edited_at";

pub struct CommentRow {
    pub id: uuid::Uuid,
    pub topic_id: uuid::Uuid,
    pub author_id: uuid::Uuid,
    pub parent_id: Option<uuid::Uuid>,
    pub body: String,
    pub created_at: OffsetDateTime,
    pub deleted_reason: Option<String>,
    pub deleted_by: Option<uuid::Uuid>,
    pub deleted_at: Option<OffsetDateTime>,
    pub deletion_penalty: i32,
    pub edited_by: Option<uuid::Uuid>,
    pub edited_at: Option<OffsetDateTime>,
}

pub fn comment_row(row: &Row) -> CommentRow {
    CommentRow {
        id: read_uuid(row, 0),
        topic_id: read_uuid(row, 1),
        author_id: read_uuid(row, 2),
        parent_id: read_opt_uuid(row, 3),
        body: row.get(4).expect("read body"),
        created_at: read_time(row, 5),
        deleted_reason: row.get(6).expect("read reason"),
        deleted_by: read_opt_uuid(row, 7),
        deleted_at: read_opt_time(row, 8),
        deletion_penalty: row.get(9).expect("read penalty"),
        edited_by: read_opt_uuid(row, 10),
        edited_at: read_opt_time(row, 11),
    }
}

pub fn to_comment(row: CommentRow) -> Comment {
    let deleted = match (&row.deleted_reason, row.deleted_by, row.deleted_at) {
        (Some(reason), Some(moderator_id), Some(deleted_at)) => Some(Deletion::new(
            UserId::new(moderator_id),
            Reason::parse(reason).expect("stored reason is valid"),
            Penalty::parse(row.deletion_penalty).unwrap_or_default(),
            deleted_at,
        )),
        _ => None,
    };
    let edited = revision(row.edited_by, row.edited_at);
    Comment::from_parts(
        CommentId::new(row.id),
        TopicId::new(row.topic_id),
        UserId::new(row.author_id),
        row.parent_id.map(CommentId::new),
        Body::parse(&row.body).expect("stored body is valid"),
        row.created_at,
        deleted,
        edited,
    )
}

async fn load_comments(db: &Db, sql: String, params: Vec<Value>) -> Vec<Comment> {
    let rows: Vec<CommentRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare comments");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(comment_row(row))
                })
                .expect("query comments");
            mapped.map(|r| r.expect("read comment")).collect()
        })
        .await;
    rows.into_iter().map(to_comment).collect()
}

#[async_trait::async_trait]
impl CommentRepository for DuckCommentRepository {
    async fn save(&self, comment: &Comment) {
        let params = vec![
            uuid_value(comment.id().as_uuid()),
            uuid_value(comment.topic_id().as_uuid()),
            uuid_value(comment.author_id().as_uuid()),
            opt_uuid(comment.parent_id().map(|p| p.as_uuid())),
            Value::Text(comment.body().as_str().to_owned()),
            time_to_value(comment.created_at()),
            penalty_value(comment.deletion()),
        ];
        self.db
            .execute(
                "INSERT INTO comments (id, topic_id, author_id, parent_id, body, created_at, \
                 deletion_penalty) VALUES (?, ?, ?, ?, ?, ?, ?)",
                params,
            )
            .await;
    }

    async fn update(&self, comment: &Comment) {
        let params = vec![
            Value::Text(comment.body().as_str().to_owned()),
            opt_text(comment.deletion().map(|d| d.reason().as_str())),
            opt_uuid(comment.deletion().map(|d| d.moderator_id().as_uuid())),
            opt_time(comment.deletion().map(|d| d.deleted_at())),
            opt_uuid(comment.revision().map(|r| r.editor_id().as_uuid())),
            opt_time(comment.revision().map(|r| r.edited_at())),
            penalty_value(comment.deletion()),
            uuid_value(comment.id().as_uuid()),
        ];
        self.db
            .execute(
                "UPDATE comments SET body = ?, deleted_reason = ?, deleted_by = ?, \
                 deleted_at = ?, edited_by = ?, edited_at = ?, deletion_penalty = ? \
                 WHERE id = ?",
                params,
            )
            .await;
    }

    async fn find_by_id(&self, id: CommentId) -> Option<Comment> {
        load_comments(
            &self.db,
            format!("SELECT {COMMENT_COLUMNS} FROM comments WHERE id = ?"),
            vec![uuid_value(id.as_uuid())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn list_by_topic(&self, topic_id: TopicId, page: Page) -> Vec<Comment> {
        let params = vec![
            uuid_value(topic_id.as_uuid()),
            limit_value(page),
            offset_value(page),
            uuid_value(topic_id.as_uuid()),
        ];
        load_comments(
            &self.db,
            format!(
                "WITH RECURSIVE roots AS (\
                 SELECT id FROM comments WHERE topic_id = ? AND parent_id IS NULL \
                 ORDER BY created_at ASC LIMIT ? OFFSET ?\
                 ), tree AS (\
                 SELECT id FROM roots \
                 UNION ALL \
                 SELECT c.id FROM comments c JOIN tree t ON c.parent_id = t.id\
                 ) \
                 SELECT {COMMENT_COLUMNS} FROM comments \
                 WHERE topic_id = ? AND id IN (SELECT id FROM tree) \
                 ORDER BY created_at ASC"
            ),
            params,
        )
        .await
    }

    async fn count_roots(&self, topic_id: TopicId) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM comments WHERE topic_id = ? AND parent_id IS NULL",
            vec![uuid_value(topic_id.as_uuid())],
        )
        .await
    }
}
