use crate::duckdb::conn::Db;
use crate::duckdb::topic::{count, opt_text, read_uuid, uuid_value};
use app::TagRepository;
use domain::{Slug, Tag, TagDescription, UserId};
use duckdb::Row;
use duckdb::types::Value;

pub struct DuckTagRepository {
    db: Db,
}

impl DuckTagRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const TAG_COLUMNS: &str = "slug, description, means";

struct TagRow {
    slug: String,
    description: Option<String>,
    means: Option<String>,
}

fn tag_row(row: &Row) -> TagRow {
    TagRow {
        slug: row.get(0).expect("read slug"),
        description: row.get(1).expect("read description"),
        means: row.get(2).expect("read means"),
    }
}

fn to_tag(row: TagRow) -> Option<Tag> {
    let slug = Slug::parse(&row.slug).ok()?;
    let description = match row.description {
        Some(raw) => Some(TagDescription::parse(&raw).ok()?),
        None => None,
    };
    let means = match row.means {
        Some(raw) => Some(Slug::parse(&raw).ok()?),
        None => None,
    };
    Some(Tag::from_parts(slug, description, means))
}

async fn load_tags(db: &Db, sql: String, params: Vec<Value>) -> Vec<Tag> {
    let rows: Vec<TagRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare tags");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(tag_row(row))
                })
                .expect("query tags");
            mapped.map(|r| r.expect("read tag")).collect()
        })
        .await;
    rows.into_iter().filter_map(to_tag).collect()
}

#[async_trait::async_trait]
impl TagRepository for DuckTagRepository {
    async fn save(&self, tag: &Tag) {
        let params = vec![
            Value::Text(tag.slug().as_str().to_owned()),
            opt_text(tag.description().map(|d| d.as_str())),
            opt_text(tag.means().map(|m| m.as_str())),
        ];
        self.db
            .execute(
                "INSERT INTO tags (slug, description, means) VALUES (?, ?, ?) \
                 ON CONFLICT (slug) \
                 DO UPDATE SET description = excluded.description, means = excluded.means",
                params,
            )
            .await;
    }

    async fn find(&self, slug: &Slug) -> Option<Tag> {
        load_tags(
            &self.db,
            format!("SELECT {TAG_COLUMNS} FROM tags WHERE slug = ?"),
            vec![Value::Text(slug.as_str().to_owned())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn list(&self) -> Vec<Tag> {
        load_tags(
            &self.db,
            format!("SELECT {TAG_COLUMNS} FROM tags ORDER BY slug"),
            Vec::new(),
        )
        .await
    }

    async fn follow(&self, user_id: UserId, slug: &Slug) {
        self.db
            .execute(
                "INSERT INTO tag_follows (user_id, slug) VALUES (?, ?) \
                 ON CONFLICT (user_id, slug) DO NOTHING",
                vec![
                    uuid_value(user_id.as_uuid()),
                    Value::Text(slug.as_str().to_owned()),
                ],
            )
            .await;
    }

    async fn unfollow(&self, user_id: UserId, slug: &Slug) {
        self.db
            .execute(
                "DELETE FROM tag_follows WHERE user_id = ? AND slug = ?",
                vec![
                    uuid_value(user_id.as_uuid()),
                    Value::Text(slug.as_str().to_owned()),
                ],
            )
            .await;
    }

    async fn is_following(&self, user_id: UserId, slug: &Slug) -> bool {
        count(
            &self.db,
            "SELECT COUNT(*) FROM tag_follows WHERE user_id = ? AND slug = ?",
            vec![
                uuid_value(user_id.as_uuid()),
                Value::Text(slug.as_str().to_owned()),
            ],
        )
        .await
            > 0
    }

    async fn followed_by(&self, user_id: UserId) -> Vec<Slug> {
        let params = vec![uuid_value(user_id.as_uuid())];
        let slugs: Vec<String> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT slug FROM tag_follows WHERE user_id = ? ORDER BY slug")
                    .expect("prepare followed tags");
                let mapped = stmt
                    .query_map(duckdb::params_from_iter(params.iter()), |row| row.get(0))
                    .expect("query followed tags");
                mapped.map(|r| r.expect("read followed tag")).collect()
            })
            .await;
        slugs
            .into_iter()
            .filter_map(|slug| Slug::parse(&slug).ok())
            .collect()
    }

    async fn followers(&self, slug: &Slug) -> Vec<UserId> {
        let params = vec![Value::Text(slug.as_str().to_owned())];
        self.db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT user_id FROM tag_follows WHERE slug = ? ORDER BY user_id")
                    .expect("prepare tag followers");
                let mapped = stmt
                    .query_map(duckdb::params_from_iter(params.iter()), |row| {
                        Ok(UserId::new(read_uuid(row, 0)))
                    })
                    .expect("query tag followers");
                mapped.map(|r| r.expect("read tag follower")).collect()
            })
            .await
    }
}
