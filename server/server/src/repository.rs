use app::UserRepository;
use domain::{Bio, Email, User, UserId, Username};
use sqlx::{FromRow, PgPool};

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    username: String,
    email: String,
    password_hash: String,
    bio: Option<String>,
}

fn to_user(row: Row) -> User {
    let bio = row
        .bio
        .as_deref()
        .and_then(|b| Bio::parse(b).expect("stored bio is valid"));
    User::from_parts(
        UserId::new(row.id),
        Username::parse(&row.username).expect("stored username is valid"),
        Email::parse(&row.email).expect("stored email is valid"),
        row.password_hash,
        bio,
    )
}

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_username(&self, username: &Username) -> Option<User> {
        sqlx::query_as::<_, Row>(
            "SELECT id, username, email, password_hash, bio FROM users WHERE username = $1",
        )
        .bind(username.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_username")
        .map(to_user)
    }

    async fn find_by_email(&self, email: &Email) -> Option<User> {
        sqlx::query_as::<_, Row>(
            "SELECT id, username, email, password_hash, bio FROM users WHERE email = $1",
        )
        .bind(email.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_email")
        .map(to_user)
    }

    async fn find_by_id(&self, id: UserId) -> Option<User> {
        sqlx::query_as::<_, Row>(
            "SELECT id, username, email, password_hash, bio FROM users WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_id")
        .map(to_user)
    }

    async fn save(&self, user: &User) {
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        )
        .bind(user.id().as_uuid())
        .bind(user.username().as_str())
        .bind(user.email().as_str())
        .bind(user.password_hash())
        .execute(&self.pool)
        .await
        .expect("insert user");
    }

    async fn update(&self, user: &User) {
        sqlx::query("UPDATE users SET username = $2, email = $3, password_hash = $4, bio = $5 WHERE id = $1")
            .bind(user.id().as_uuid())
            .bind(user.username().as_str())
            .bind(user.email().as_str())
            .bind(user.password_hash())
            .bind(user.bio().map(Bio::as_str))
            .execute(&self.pool)
            .await
            .expect("update user");
    }
}
