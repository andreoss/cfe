use app::{
    ActivityRepository, AvatarRepository, BookmarkRepository, CommentRepository,
    EnforcementRepository, NotificationRepository, PollRepository, ReactionRepository,
    SearchRepository, SectionRepository, SessionRepository, TopicRepository, UserRepository,
};
use std::sync::Arc;

pub trait Backend: Send + Sync {
    fn users(&self) -> Arc<dyn UserRepository + Send + Sync>;
    fn sessions(&self) -> Arc<dyn SessionRepository + Send + Sync>;
    fn sections(&self) -> Arc<dyn SectionRepository + Send + Sync>;
    fn topics(&self) -> Arc<dyn TopicRepository + Send + Sync>;
    fn comments(&self) -> Arc<dyn CommentRepository + Send + Sync>;
    fn notifications(&self) -> Arc<dyn NotificationRepository + Send + Sync>;
    fn bookmarks(&self) -> Arc<dyn BookmarkRepository + Send + Sync>;
    fn reactions(&self) -> Arc<dyn ReactionRepository + Send + Sync>;
    fn polls(&self) -> Arc<dyn PollRepository + Send + Sync>;
    fn avatars(&self) -> Arc<dyn AvatarRepository + Send + Sync>;
    fn enforcement(&self) -> Arc<dyn EnforcementRepository + Send + Sync>;
    fn search(&self) -> Arc<dyn SearchRepository + Send + Sync>;
    fn activity(&self) -> Arc<dyn ActivityRepository + Send + Sync>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum Vendor {
    Postgres,
    MySql,
    DuckDb,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnknownVendor;

impl Vendor {
    pub fn from_url(url: &str) -> Result<Self, UnknownVendor> {
        let scheme = url.split("://").next().unwrap_or_default();
        match scheme {
            "postgres" | "postgresql" => Ok(Self::Postgres),
            "mysql" => Ok(Self::MySql),
            "duckdb" => Ok(Self::DuckDb),
            _ => Err(UnknownVendor),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::MySql => "mysql",
            Self::DuckDb => "duckdb",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_each_supported_scheme() {
        assert_eq!(
            Vendor::from_url("postgres://user@host/db"),
            Ok(Vendor::Postgres)
        );
        assert_eq!(
            Vendor::from_url("postgresql://user@host/db"),
            Ok(Vendor::Postgres)
        );
        assert_eq!(Vendor::from_url("mysql://user@host/db"), Ok(Vendor::MySql));
        assert_eq!(Vendor::from_url("duckdb://data.db"), Ok(Vendor::DuckDb));
    }

    #[test]
    fn refuses_an_unknown_scheme_rather_than_guessing() {
        assert_eq!(Vendor::from_url("sqlite://data.db"), Err(UnknownVendor));
        assert_eq!(Vendor::from_url("http://host/db"), Err(UnknownVendor));
        assert_eq!(Vendor::from_url("data.db"), Err(UnknownVendor));
        assert_eq!(Vendor::from_url(""), Err(UnknownVendor));
    }

    #[test]
    fn names_itself_for_logging() {
        assert_eq!(Vendor::Postgres.as_str(), "postgres");
        assert_eq!(Vendor::MySql.as_str(), "mysql");
        assert_eq!(Vendor::DuckDb.as_str(), "duckdb");
    }
}
