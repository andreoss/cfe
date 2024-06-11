use crate::sqlite::conn::Db;
use crate::sqlite::topic::{time_to_value, uuid_value};
use app::AvatarRepository;
use domain::{Avatar, ImageFormat, UserId};
use rusqlite::types::Value;
use time::OffsetDateTime;

pub struct SqliteAvatarRepository {
    db: Db,
}

impl SqliteAvatarRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl AvatarRepository for SqliteAvatarRepository {
    async fn save(&self, user_id: UserId, avatar: &Avatar) {
        let params = vec![
            uuid_value(user_id.as_uuid()),
            Value::Blob(avatar.bytes().to_vec()),
            Value::Text(avatar.format().content_type().to_owned()),
            time_to_value(OffsetDateTime::now_utc()),
        ];
        self.db
            .execute(
                "INSERT INTO avatars (user_id, bytes, content_type, updated_at) \
                 VALUES (?, ?, ?, ?) \
                 ON CONFLICT (user_id) DO UPDATE SET bytes = EXCLUDED.bytes, \
                 content_type = EXCLUDED.content_type, updated_at = EXCLUDED.updated_at",
                params,
            )
            .await;
    }

    async fn find_by_user(&self, user_id: UserId) -> Option<Avatar> {
        let id = user_id.as_uuid();
        let rows: Vec<(Vec<u8>, String)> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT bytes, content_type FROM avatars WHERE user_id = ?")
                    .expect("prepare avatar");
                let mapped = stmt
                    .query_map([uuid_value(id)], |row| {
                        Ok((
                            row.get::<_, Vec<u8>>(0).expect("read bytes"),
                            row.get::<_, String>(1).expect("read content type"),
                        ))
                    })
                    .expect("query avatar");
                mapped.map(|r| r.expect("read avatar")).collect()
            })
            .await;
        let (bytes, content_type) = rows.into_iter().next()?;
        let format = ImageFormat::parse(&content_type)?;
        Some(Avatar::from_parts(bytes, format))
    }

    async fn delete(&self, user_id: UserId) {
        self.db
            .execute(
                "DELETE FROM avatars WHERE user_id = ?",
                vec![uuid_value(user_id.as_uuid())],
            )
            .await;
    }
}
