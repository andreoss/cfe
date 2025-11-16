use app::UserRepository;
use domain::{Bio, Email, Role, User, UserId, Username};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

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
    role: String,
    deregistered_at: Option<OffsetDateTime>,
}

fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Moderator => "moderator",
    }
}

fn role_from_str(raw: &str) -> Role {
    match raw {
        "moderator" => Role::Moderator,
        _ => Role::User,
    }
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
        role_from_str(&row.role),
        row.deregistered_at,
    )
}

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_username(&self, username: &Username) -> Option<User> {
        sqlx::query_as::<_, Row>(
            "SELECT id, username, email, password_hash, bio, role, deregistered_at FROM users WHERE username = $1",
        )
        .bind(username.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_username")
        .map(to_user)
    }

    async fn find_by_email(&self, email: &Email) -> Option<User> {
        sqlx::query_as::<_, Row>(
            "SELECT id, username, email, password_hash, bio, role, deregistered_at FROM users WHERE email = $1",
        )
        .bind(email.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_email")
        .map(to_user)
    }

    async fn find_by_id(&self, id: UserId) -> Option<User> {
        sqlx::query_as::<_, Row>(
            "SELECT id, username, email, password_hash, bio, role, deregistered_at FROM users WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_id")
        .map(to_user)
    }

    async fn save(&self, user: &User) {
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user.id().as_uuid())
        .bind(user.username().as_str())
        .bind(user.email().as_str())
        .bind(user.password_hash())
        .bind(role_to_str(user.role()))
        .execute(&self.pool)
        .await
        .expect("insert user");
    }

    async fn update(&self, user: &User) {
        sqlx::query(
            "UPDATE users SET username = $2, email = $3, password_hash = $4, bio = $5, \
             role = $6, deregistered_at = $7 WHERE id = $1",
        )
        .bind(user.id().as_uuid())
        .bind(user.username().as_str())
        .bind(user.email().as_str())
        .bind(user.password_hash())
        .bind(user.bio().map(Bio::as_str))
        .bind(role_to_str(user.role()))
        .bind(user.deregistered_at())
        .execute(&self.pool)
        .await
        .expect("update user");
    }

    async fn count(&self) -> u64 {
        let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .expect("query count users");
        count as u64
    }
}
