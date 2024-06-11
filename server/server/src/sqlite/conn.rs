use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &str) -> Self {
        let conn = if path.is_empty() || path == ":memory:" {
            Connection::open_in_memory().expect("open database")
        } else {
            Connection::open(path).expect("open database")
        };
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub async fn call<T, F>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> T + Send + 'static,
    {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            f(&guard)
        })
        .await
        .expect("database task")
    }

    pub async fn execute(&self, sql: &'static str, params: Vec<rusqlite::types::Value>) {
        self.call(move |conn| {
            conn.execute(sql, rusqlite::params_from_iter(params.iter()))
                .expect("execute statement");
        })
        .await
    }
}

pub fn path_from_url(url: &str) -> String {
    url.strip_prefix("sqlite://").unwrap_or(url).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_the_scheme_to_get_a_path() {
        assert_eq!(path_from_url("sqlite://forum.db"), "forum.db");
        assert_eq!(path_from_url("sqlite:///tmp/forum.db"), "/tmp/forum.db");
        assert_eq!(path_from_url("sqlite://:memory:"), ":memory:");
    }

    #[tokio::test]
    async fn runs_statements_on_a_blocking_task() {
        let db = Db::open(":memory:");
        db.call(|conn| {
            conn.execute_batch("CREATE TABLE t (n INTEGER); INSERT INTO t VALUES (7)")
                .expect("create");
        })
        .await;
        let n: i32 = db
            .call(|conn| {
                conn.query_row("SELECT n FROM t", [], |row| row.get(0))
                    .expect("query")
            })
            .await;
        assert_eq!(n, 7);
    }

    #[tokio::test]
    async fn shares_one_connection_across_clones() {
        let db = Db::open(":memory:");
        db.call(|conn| {
            conn.execute_batch("CREATE TABLE t (n INTEGER)")
                .expect("create");
        })
        .await;
        let other = db.clone();
        other
            .call(|conn| {
                conn.execute_batch("INSERT INTO t VALUES (1)")
                    .expect("insert");
            })
            .await;
        let count: i64 = db
            .call(|conn| {
                conn.query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
                    .expect("count")
            })
            .await;
        assert_eq!(count, 1);
    }
}
