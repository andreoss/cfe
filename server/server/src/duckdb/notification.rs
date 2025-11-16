use crate::duckdb::conn::Db;
use crate::duckdb::topic::{opt_time, read_opt_time, read_time, read_uuid, time_to_value, uuid_value};
use app::NotificationRepository;
use domain::{CommentId, Notification, NotificationId, TopicId, UserId};
use duckdb::Row;
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckNotificationRepository {
    db: Db,
}

impl DuckNotificationRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const NOTIFICATION_COLUMNS: &str =
    "id, recipient_id, actor_id, topic_id, comment_id, created_at, read_at";

struct NotificationRow {
    id: uuid::Uuid,
    recipient_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    topic_id: uuid::Uuid,
    comment_id: uuid::Uuid,
    created_at: OffsetDateTime,
    read_at: Option<OffsetDateTime>,
}

fn notification_row(row: &Row) -> NotificationRow {
    NotificationRow {
        id: read_uuid(row, 0),
        recipient_id: read_uuid(row, 1),
        actor_id: read_uuid(row, 2),
        topic_id: read_uuid(row, 3),
        comment_id: read_uuid(row, 4),
        created_at: read_time(row, 5),
        read_at: read_opt_time(row, 6),
    }
}

fn to_notification(row: NotificationRow) -> Notification {
    Notification::from_parts(
        NotificationId::new(row.id),
        UserId::new(row.recipient_id),
        UserId::new(row.actor_id),
        TopicId::new(row.topic_id),
        CommentId::new(row.comment_id),
        row.created_at,
        row.read_at,
    )
}

async fn load_notifications(db: &Db, sql: String, params: Vec<Value>) -> Vec<Notification> {
    let rows: Vec<NotificationRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare notifications");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(notification_row(row))
                })
                .expect("query notifications");
            mapped.map(|r| r.expect("read notification")).collect()
        })
        .await;
    rows.into_iter().map(to_notification).collect()
}

#[async_trait::async_trait]
impl NotificationRepository for DuckNotificationRepository {
    async fn save(&self, notification: &Notification) {
        let params = vec![
            uuid_value(notification.id().as_uuid()),
            uuid_value(notification.recipient_id().as_uuid()),
            uuid_value(notification.actor_id().as_uuid()),
            uuid_value(notification.topic_id().as_uuid()),
            uuid_value(notification.comment_id().as_uuid()),
            time_to_value(notification.created_at()),
        ];
        self.db
            .execute(
                "INSERT INTO notifications \
                 (id, recipient_id, actor_id, topic_id, comment_id, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                params,
            )
            .await;
    }

    async fn update(&self, notification: &Notification) {
        let params = vec![
            opt_time(notification.read_at()),
            uuid_value(notification.id().as_uuid()),
        ];
        self.db
            .execute("UPDATE notifications SET read_at = ? WHERE id = ?", params)
            .await;
    }

    async fn find_by_id(&self, id: NotificationId) -> Option<Notification> {
        load_notifications(
            &self.db,
            format!("SELECT {NOTIFICATION_COLUMNS} FROM notifications WHERE id = ?"),
            vec![uuid_value(id.as_uuid())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn list_by_recipient(&self, recipient_id: UserId) -> Vec<Notification> {
        load_notifications(
            &self.db,
            format!(
                "SELECT {NOTIFICATION_COLUMNS} FROM notifications \
                 WHERE recipient_id = ? ORDER BY created_at DESC LIMIT 50"
            ),
            vec![uuid_value(recipient_id.as_uuid())],
        )
        .await
    }

    async fn count_unread(&self, recipient_id: UserId) -> u64 {
        let params = vec![uuid_value(recipient_id.as_uuid())];
        let count: i64 = self
            .db
            .call(move |conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM notifications \
                     WHERE recipient_id = ? AND read_at IS NULL",
                    duckdb::params_from_iter(params.iter()),
                    |row| row.get(0),
                )
                .expect("count unread notifications")
            })
            .await;
        count as u64
    }
}
