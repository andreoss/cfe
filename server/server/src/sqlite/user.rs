use crate::sqlite::conn::Db;
use crate::sqlite::topic::{
    int_value, opt_text, opt_time, read_opt_time, read_time, read_uuid, time_to_value, uuid_value,
};
use app::UserRepository;
use domain::{Bio, Email, Role, Score, User, UserId, Username};
use rusqlite::Row;
use rusqlite::types::Value;
use time::OffsetDateTime;

pub struct SqliteUserRepository {
    db: Db,
}

impl SqliteUserRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const USER_COLUMNS: &str = "id, username, email, password_hash, bio, role, deregistered_at, \
     confirmed_at, score, created_at";

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
    created_at: OffsetDateTime,
}

fn user_row(row: &Row) -> UserRow {
    UserRow {
        id: read_uuid(row, 0),
        username: row.get(1).expect("read username"),
        email: row.get(2).expect("read email"),
        password_hash: row.get(3).expect("read password hash"),
        bio: row.get(4).expect("read bio"),
        role: row.get(5).expect("read role"),
        deregistered_at: read_opt_time(row, 6),
        confirmed_at: read_opt_time(row, 7),
        score: row.get(8).expect("read score"),
        created_at: read_time(row, 9),
    }
}

fn role_to_str(role: Role) -> &'static str {
    role.as_str()
}

fn role_from_str(raw: &str) -> Role {
    Role::parse(raw).unwrap_or(Role::User)
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
        row.created_at,
    )
}

async fn load_users(db: &Db, sql: String, params: Vec<Value>) -> Vec<User> {
    let rows: Vec<UserRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare users");
            let mapped = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    Ok(user_row(row))
                })
                .expect("query users");
            mapped.map(|r| r.expect("read user")).collect()
        })
        .await;
    rows.into_iter().map(to_user).collect()
}

async fn find_user(db: &Db, sql: String, params: Vec<Value>) -> Option<User> {
    load_users(db, sql, params).await.into_iter().next()
}

#[async_trait::async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_username(&self, username: &Username) -> Option<User> {
        find_user(
            &self.db,
            format!("SELECT {USER_COLUMNS} FROM users WHERE username = ?"),
            vec![Value::Text(username.as_str().to_owned())],
        )
        .await
    }

    async fn find_by_email(&self, email: &Email) -> Option<User> {
        find_user(
            &self.db,
            format!("SELECT {USER_COLUMNS} FROM users WHERE email = ?"),
            vec![Value::Text(email.as_str().to_owned())],
        )
        .await
    }

    async fn find_by_id(&self, id: UserId) -> Option<User> {
        find_user(
            &self.db,
            format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?"),
            vec![uuid_value(id.as_uuid())],
        )
        .await
    }

    async fn save(&self, user: &User) {
        let params = vec![
            uuid_value(user.id().as_uuid()),
            Value::Text(user.username().as_str().to_owned()),
            Value::Text(user.email().as_str().to_owned()),
            Value::Text(user.password_hash().to_owned()),
            Value::Text(role_to_str(user.role()).to_owned()),
            int_value(user.score().value()),
        ];
        self.db
            .execute(
                "INSERT INTO users (id, username, email, password_hash, role, score) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                params,
            )
            .await;
    }

    async fn update(&self, user: &User) {
        let params = vec![
            Value::Text(user.username().as_str().to_owned()),
            Value::Text(user.email().as_str().to_owned()),
            Value::Text(user.password_hash().to_owned()),
            opt_text(user.bio().map(Bio::as_str)),
            Value::Text(role_to_str(user.role()).to_owned()),
            opt_time(user.deregistered_at()),
            opt_time(user.confirmed_at()),
            int_value(user.score().value()),
            uuid_value(user.id().as_uuid()),
        ];
        self.db
            .execute(
                "UPDATE users SET username = ?, email = ?, password_hash = ?, bio = ?, \
                 role = ?, deregistered_at = ?, confirmed_at = ?, score = ? WHERE id = ?",
                params,
            )
            .await;
    }

    async fn count(&self) -> u64 {
        let count: i64 = self
            .db
            .call(|conn| {
                conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
                    .expect("count users")
            })
            .await;
        count as u64
    }

    async fn find_at_or_below_score(&self, score: i32) -> Vec<User> {
        load_users(
            &self.db,
            format!("SELECT {USER_COLUMNS} FROM users WHERE score <= ?"),
            vec![int_value(score)],
        )
        .await
    }

    async fn find_unconfirmed_before(&self, cutoff: OffsetDateTime) -> Vec<User> {
        load_users(
            &self.db,
            format!(
                "SELECT {USER_COLUMNS} FROM users WHERE confirmed_at IS NULL AND created_at < ?"
            ),
            vec![time_to_value(cutoff)],
        )
        .await
    }
}
