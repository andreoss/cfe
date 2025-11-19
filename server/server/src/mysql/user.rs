use app::UserRepository;
use domain::{Bio, Email, Role, Score, User, UserId, Username};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlUserRepository {
    pool: MySqlPool,
}

impl MySqlUserRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

const USER_COLUMNS: &str =
    "id, username, email, password_hash, bio, role, deregistered_at, confirmed_at, score";

#[derive(FromRow)]
struct UserRow {
    id: uuid::Uuid,
    username: String,
    email: String,
    password_hash: String,
    bio: Option<String>,
    role: String,
    deregistered_at: Option<OffsetDateTime>,
    confirmed_at: Option<OffsetDateTime>,
    score: i32,
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

fn to_user(row: UserRow) -> User {
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
        row.confirmed_at,
        Score::of(row.score),
    )
}

#[async_trait::async_trait]
impl UserRepository for MySqlUserRepository {
    async fn find_by_username(&self, username: &Username) -> Option<User> {
        sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLUMNS} FROM users WHERE username = ?"
        ))
        .bind(username.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_username")
        .map(to_user)
    }

    async fn find_by_email(&self, email: &Email) -> Option<User> {
        sqlx::query_as::<_, UserRow>(&format!("SELECT {USER_COLUMNS} FROM users WHERE email = ?"))
            .bind(email.as_str())
            .fetch_optional(&self.pool)
            .await
            .expect("query find_by_email")
            .map(to_user)
    }

    async fn find_by_id(&self, id: UserId) -> Option<User> {
        sqlx::query_as::<_, UserRow>(&format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?"))
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .expect("query find_by_id")
            .map(to_user)
    }

    async fn save(&self, user: &User) {
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role, score) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(user.id().as_uuid())
        .bind(user.username().as_str())
        .bind(user.email().as_str())
        .bind(user.password_hash())
        .bind(role_to_str(user.role()))
        .bind(user.score().value())
        .execute(&self.pool)
        .await
        .expect("insert user");
    }

    async fn update(&self, user: &User) {
        sqlx::query(
            "UPDATE users SET username = ?, email = ?, password_hash = ?, bio = ?, \
             role = ?, deregistered_at = ?, confirmed_at = ?, score = ? WHERE id = ?",
        )
        .bind(user.username().as_str())
        .bind(user.email().as_str())
        .bind(user.password_hash())
        .bind(user.bio().map(Bio::as_str))
        .bind(role_to_str(user.role()))
        .bind(user.deregistered_at())
        .bind(user.confirmed_at())
        .bind(user.score().value())
        .bind(user.id().as_uuid())
        .execute(&self.pool)
        .await
        .expect("update user");
    }

    async fn count(&self) -> u64 {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .expect("query count users");
        count as u64
    }
}
