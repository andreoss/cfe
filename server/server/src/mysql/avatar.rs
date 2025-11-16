use app::AvatarRepository;
use domain::{Avatar, ImageFormat, UserId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlAvatarRepository {
    pool: MySqlPool,
}

impl MySqlAvatarRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct Row {
    bytes: Vec<u8>,
    content_type: String,
}

#[async_trait::async_trait]
impl AvatarRepository for MySqlAvatarRepository {
    async fn save(&self, user_id: UserId, avatar: &Avatar) {
        sqlx::query(
            "INSERT INTO avatars (user_id, bytes, content_type, updated_at) \
             VALUES (?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE bytes = VALUES(bytes), \
             content_type = VALUES(content_type), updated_at = VALUES(updated_at)",
        )
        .bind(user_id.as_uuid())
        .bind(avatar.bytes().to_vec())
        .bind(avatar.format().content_type())
        .bind(OffsetDateTime::now_utc())
        .execute(&self.pool)
        .await
        .expect("insert avatar");
    }

    async fn find_by_user(&self, user_id: UserId) -> Option<Avatar> {
        let row =
            sqlx::query_as::<_, Row>("SELECT bytes, content_type FROM avatars WHERE user_id = ?")
                .bind(user_id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .expect("query avatar")?;
        let format = ImageFormat::parse(&row.content_type)?;
        Some(Avatar::from_parts(row.bytes, format))
    }

    async fn delete(&self, user_id: UserId) {
        sqlx::query("DELETE FROM avatars WHERE user_id = ?")
            .bind(user_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete avatar");
    }
}
