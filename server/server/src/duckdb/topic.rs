use crate::duckdb::conn::Db;
use app::TopicRepository;
use domain::{
    Body, Deletion, GroupId, Page, PostScore, Reason, Revision, SectionId, Slug, TagSet, Title,
    Topic, TopicId, UserId,
};
use duckdb::Row;
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckTopicRepository {
    db: Db,
}

impl DuckTopicRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

pub const TOPIC_COLUMNS: &str = "id, section_id, author_id, title, body, created_at, postscore, \
    deleted_reason, deleted_by, deleted_at, edited_by, edited_at, group_id, pending, draft, \
    sticky, off_front, resolved, minor";
pub struct TopicRow {
    pub id: uuid::Uuid,
    pub section_id: uuid::Uuid,
    pub author_id: uuid::Uuid,
    pub title: String,
    pub body: String,
    pub created_at: OffsetDateTime,
    pub postscore: i32,
    pub deleted_reason: Option<String>,
    pub deleted_by: Option<uuid::Uuid>,
    pub deleted_at: Option<OffsetDateTime>,
    pub edited_by: Option<uuid::Uuid>,
    pub edited_at: Option<OffsetDateTime>,
    pub group_id: Option<uuid::Uuid>,
    pub pending: bool,
    pub draft: bool,
    pub sticky: bool,
    pub off_front: bool,
    pub resolved: bool,
    pub minor: bool,
}
pub fn micros_to_time(micros: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(micros as i128 * 1_000).expect("stored timestamp")
}

pub fn time_to_value(at: OffsetDateTime) -> Value {
    Value::Timestamp(
        duckdb::types::TimeUnit::Microsecond,
        (at.unix_timestamp_nanos() / 1_000) as i64,
    )
}

pub fn opt_time(at: Option<OffsetDateTime>) -> Value {
    match at {
        Some(at) => time_to_value(at),
        None => Value::Null,
    }
}

pub fn uuid_value(id: uuid::Uuid) -> Value {
    Value::Text(id.to_string())
}

pub fn opt_uuid(id: Option<uuid::Uuid>) -> Value {
    match id {
        Some(id) => uuid_value(id),
        None => Value::Null,
    }
}

pub fn limit_value(page: Page) -> Value {
    Value::BigInt(page.limit() as i64)
}

pub fn offset_value(page: Page) -> Value {
    Value::BigInt(page.offset() as i64)
}

pub async fn count(db: &Db, sql: &'static str, params: Vec<Value>) -> u64 {
    let total: i64 = db
        .call(move |conn| {
            conn.query_row(sql, duckdb::params_from_iter(params.iter()), |row| {
                row.get(0)
            })
            .expect("count rows")
        })
        .await;
    total as u64
}

pub fn tagged_columns() -> String {
    TOPIC_COLUMNS
        .split(", ")
        .map(|c| format!("t.{c}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn opt_text(text: Option<&str>) -> Value {
    match text {
        Some(text) => Value::Text(text.to_owned()),
        None => Value::Null,
    }
}

pub fn read_uuid(row: &Row, index: usize) -> uuid::Uuid {
    let raw: String = row.get(index).expect("read uuid");
    uuid::Uuid::parse_str(&raw).expect("stored uuid is valid")
}

pub fn read_opt_uuid(row: &Row, index: usize) -> Option<uuid::Uuid> {
    let raw: Option<String> = row.get(index).expect("read uuid");
    raw.map(|r| uuid::Uuid::parse_str(&r).expect("stored uuid is valid"))
}

pub fn read_time(row: &Row, index: usize) -> OffsetDateTime {
    let micros: i64 = row.get(index).expect("read timestamp");
    micros_to_time(micros)
}

pub fn read_opt_time(row: &Row, index: usize) -> Option<OffsetDateTime> {
    let micros: Option<i64> = row.get(index).expect("read timestamp");
    micros.map(micros_to_time)
}

pub fn topic_row(row: &Row) -> TopicRow {
    TopicRow {
        id: read_uuid(row, 0),
        section_id: read_uuid(row, 1),
        author_id: read_uuid(row, 2),
        title: row.get(3).expect("read title"),
        body: row.get(4).expect("read body"),
        created_at: read_time(row, 5),
        postscore: row.get(6).expect("read postscore"),
        deleted_reason: row.get(7).expect("read reason"),
        deleted_by: read_opt_uuid(row, 8),
        deleted_at: read_opt_time(row, 9),
        edited_by: read_opt_uuid(row, 10),
        edited_at: read_opt_time(row, 11),
        group_id: read_opt_uuid(row, 12),
        pending: row.get(13).expect("read pending"),
        draft: row.get(14).expect("read draft"),
        sticky: row.get(15).expect("read sticky"),
        off_front: row.get(16).expect("read off front"),
        resolved: row.get(17).expect("read resolved"),
        minor: row.get(18).expect("read minor"),
    }
}

pub fn revision(by: Option<uuid::Uuid>, at: Option<OffsetDateTime>) -> Option<Revision> {
    match (by, at) {
        (Some(editor_id), Some(edited_at)) => {
            Some(Revision::new(UserId::new(editor_id), edited_at))
        }
        _ => None,
    }
}

pub fn edit_revision(
    by: Option<uuid::Uuid>,
    at: Option<OffsetDateTime>,
    minor: bool,
) -> Option<Revision> {
    match (by, at) {
        (Some(editor_id), Some(edited_at)) if minor => {
            Some(Revision::minor(UserId::new(editor_id), edited_at))
        }
        (Some(editor_id), Some(edited_at)) => {
            Some(Revision::new(UserId::new(editor_id), edited_at))
        }
        _ => None,
    }
}

pub fn is_minor(topic: &Topic) -> bool {
    topic.revision().map(|r| r.is_minor()).unwrap_or(false)
}

pub fn to_topic(row: TopicRow, tags: TagSet) -> Topic {
    let deleted = match (&row.deleted_reason, row.deleted_by, row.deleted_at) {
        (Some(reason), Some(moderator_id), Some(deleted_at)) => Some(Deletion::new(
            UserId::new(moderator_id),
            Reason::parse(reason).expect("stored reason is valid"),
            deleted_at,
        )),
        _ => None,
    };
    let edited = edit_revision(row.edited_by, row.edited_at, row.minor);
    Topic::from_parts(
        TopicId::new(row.id),
        SectionId::new(row.section_id),
        UserId::new(row.author_id),
        Title::parse(&row.title).expect("stored title is valid"),
        Body::parse(&row.body).expect("stored body is valid"),
        tags,
        row.created_at,
        deleted,
        edited,
        row.group_id.map(GroupId::new),
        row.pending,
    )
    .with_postscore(PostScore::from_db(row.postscore))
    .with_lifecycle(row.draft, row.sticky, row.off_front, row.resolved)
}

pub async fn tags_for(db: &Db, topic_id: uuid::Uuid) -> TagSet {
    let tags: Vec<String> = db
        .call(move |conn| {
            let mut stmt = conn
                .prepare("SELECT tag FROM topic_tags WHERE topic_id = ? ORDER BY position")
                .expect("prepare tags");
            let rows = stmt
                .query_map([uuid_value(topic_id)], |row| row.get::<_, String>(0))
                .expect("query tags");
            rows.map(|r| r.expect("read tag")).collect()
        })
        .await;
    TagSet::parse(&tags).expect("stored tags are valid")
}

async fn replace_tags(db: &Db, topic: &Topic) {
    let id = topic.id().as_uuid();
    let tags: Vec<String> = topic
        .tags()
        .as_slice()
        .iter()
        .map(|t| t.as_str().to_owned())
        .collect();
    db.call(move |conn| {
        conn.execute(
            "DELETE FROM topic_tags WHERE topic_id = ?",
            [uuid_value(id)],
        )
        .expect("clear tags");
        for (position, tag) in tags.iter().enumerate() {
            conn.execute(
                "INSERT INTO topic_tags (topic_id, tag, position) VALUES (?, ?, ?)",
                duckdb::params![uuid_value(id), tag.clone(), position as i32],
            )
            .expect("insert tag");
        }
    })
    .await;
}

pub async fn load_topics(db: &Db, sql: String, params: Vec<Value>) -> Vec<Topic> {
    let rows: Vec<TopicRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare topics");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(topic_row(row))
                })
                .expect("query topics");
            mapped.map(|r| r.expect("read topic")).collect()
        })
        .await;
    let mut topics = Vec::with_capacity(rows.len());
    for row in rows {
        let tags = tags_for(db, row.id).await;
        topics.push(to_topic(row, tags));
    }
    topics
}

#[async_trait::async_trait]
impl TopicRepository for DuckTopicRepository {
    async fn all_for_archive(&self) -> Vec<Topic> {
        load_topics(
            &self.db,
            format!("SELECT {TOPIC_COLUMNS} FROM topics ORDER BY created_at DESC"),
            Vec::new(),
        )
        .await
    }

    async fn list_between(
        &self,
        from: OffsetDateTime,
        until: OffsetDateTime,
        page: Page,
    ) -> Vec<Topic> {
        load_topics(
            &self.db,
            format!(
                "SELECT {TOPIC_COLUMNS} FROM topics \
                 WHERE created_at >= ? AND created_at < ? \
                 ORDER BY created_at DESC LIMIT ? OFFSET ?"
            ),
            vec![
                time_to_value(from),
                time_to_value(until),
                limit_value(page),
                offset_value(page),
            ],
        )
        .await
    }

    async fn count_between(&self, from: OffsetDateTime, until: OffsetDateTime) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM topics WHERE created_at >= ? AND created_at < ?",
            vec![time_to_value(from), time_to_value(until)],
        )
        .await
    }

    async fn save(&self, topic: &Topic) {
        let params = vec![
            uuid_value(topic.id().as_uuid()),
            uuid_value(topic.section_id().as_uuid()),
            uuid_value(topic.author_id().as_uuid()),
            Value::Text(topic.title().as_str().to_owned()),
            Value::Text(topic.body().as_str().to_owned()),
            time_to_value(topic.created_at()),
            opt_uuid(topic.group_id().map(|g| g.as_uuid())),
            Value::Boolean(topic.is_pending()),
            Value::Boolean(topic.is_draft()),
            Value::Boolean(topic.is_sticky()),
            Value::Boolean(topic.is_off_front()),
            Value::Boolean(topic.is_resolved()),
            Value::Boolean(is_minor(topic)),
        ];
        self.db
            .execute(
                "INSERT INTO topics (id, section_id, author_id, title, body, created_at, \
                 group_id, pending, draft, sticky, off_front, resolved, minor) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params,
            )
            .await;
        replace_tags(&self.db, topic).await;
    }

    async fn update(&self, topic: &Topic) {
        let params = vec![
            Value::Text(topic.title().as_str().to_owned()),
            Value::Text(topic.body().as_str().to_owned()),
            Value::BigInt(topic.postscore().to_db() as i64),
            opt_text(topic.deletion().map(|d| d.reason().as_str())),
            opt_uuid(topic.deletion().map(|d| d.moderator_id().as_uuid())),
            opt_time(topic.deletion().map(|d| d.deleted_at())),
            opt_uuid(topic.revision().map(|r| r.editor_id().as_uuid())),
            opt_time(topic.revision().map(|r| r.edited_at())),
            opt_uuid(topic.group_id().map(|g| g.as_uuid())),
            Value::Boolean(topic.is_pending()),
            Value::Boolean(topic.is_draft()),
            Value::Boolean(topic.is_sticky()),
            Value::Boolean(topic.is_off_front()),
            Value::Boolean(topic.is_resolved()),
            Value::Boolean(is_minor(topic)),
            uuid_value(topic.id().as_uuid()),
        ];
        self.db
            .execute(
                "UPDATE topics SET title = ?, body = ?, postscore = ?, deleted_reason = ?, \
                 deleted_by = ?, deleted_at = ?, edited_by = ?, edited_at = ?, group_id = ?, \
                 pending = ?, draft = ?, sticky = ?, off_front = ?, resolved = ?, minor = ? \
                 WHERE id = ?",
                params,
            )
            .await;
        replace_tags(&self.db, topic).await;
    }

    async fn find_by_id(&self, id: TopicId) -> Option<Topic> {
        let found = load_topics(
            &self.db,
            format!("SELECT {TOPIC_COLUMNS} FROM topics WHERE id = ?"),
            vec![uuid_value(id.as_uuid())],
        )
        .await;
        found.into_iter().next()
    }

    async fn list_by_section(&self, section_id: SectionId, page: Page) -> Vec<Topic> {
        load_topics(
            &self.db,
            format!(
                "SELECT {TOPIC_COLUMNS} FROM topics \
                 WHERE section_id = ? AND deleted_at IS NULL \
                 ORDER BY sticky DESC, created_at DESC LIMIT ? OFFSET ?"
            ),
            vec![
                uuid_value(section_id.as_uuid()),
                limit_value(page),
                offset_value(page),
            ],
        )
        .await
    }

    async fn count_by_section(&self, section_id: SectionId) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM topics WHERE section_id = ? AND deleted_at IS NULL",
            vec![uuid_value(section_id.as_uuid())],
        )
        .await
    }

    async fn list_by_tag(&self, tag: &Slug, page: Page) -> Vec<Topic> {
        let columns = tagged_columns();
        load_topics(
            &self.db,
            format!(
                "SELECT {columns} FROM topics t JOIN topic_tags g ON g.topic_id = t.id \
                 WHERE g.tag = ? AND t.deleted_at IS NULL \
                 ORDER BY t.created_at DESC LIMIT ? OFFSET ?"
            ),
            vec![
                Value::Text(tag.as_str().to_owned()),
                limit_value(page),
                offset_value(page),
            ],
        )
        .await
    }

    async fn count_by_tag(&self, tag: &Slug) -> u64 {
        count(
            &self.db,
            "SELECT COUNT(*) FROM topics t JOIN topic_tags g ON g.topic_id = t.id \
             WHERE g.tag = ? AND t.deleted_at IS NULL",
            vec![Value::Text(tag.as_str().to_owned())],
        )
        .await
    }
}
