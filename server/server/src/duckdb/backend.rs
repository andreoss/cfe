use crate::backend::Backend;
use crate::duckdb::abuse::DuckAbuseRepository;
use crate::duckdb::activity::DuckActivityRepository;
use crate::duckdb::avatar::DuckAvatarRepository;
use crate::duckdb::bookmark::DuckBookmarkRepository;
use crate::duckdb::comment::DuckCommentRepository;
use crate::duckdb::conn::{Db, path_from_url};
use crate::duckdb::enforcement::DuckEnforcementRepository;
use crate::duckdb::group::DuckGroupRepository;
use crate::duckdb::mail_token::DuckMailTokenRepository;
use crate::duckdb::notification::DuckNotificationRepository;
use crate::duckdb::poll::DuckPollRepository;
use crate::duckdb::reaction::DuckReactionRepository;
use crate::duckdb::remark::DuckRemarkRepository;
use crate::duckdb::report::DuckReportRepository;
use crate::duckdb::search::DuckSearchRepository;
use crate::duckdb::section::DuckSectionRepository;
use crate::duckdb::session::DuckSessionRepository;
use crate::duckdb::topic::DuckTopicRepository;
use crate::duckdb::user::DuckUserRepository;
use crate::duckdb::watch::DuckWatchRepository;
use app::{
    AbuseRepository, ActivityRepository, AvatarRepository, BookmarkRepository, CommentRepository,
    EnforcementRepository, GroupRepository, MailTokenRepository, NotificationRepository,
    PollRepository, ReactionRepository, RemarkRepository, ReportRepository, SearchRepository,
    SectionRepository, SessionRepository, TopicRepository, UserRepository, WatchRepository,
};
use std::sync::Arc;

pub struct DuckDbBackend {
    db: Db,
}

const SCHEMA: &str = include_str!("../../migrations_duckdb/0001_schema.sql");

impl DuckDbBackend {
    pub async fn connect(url: &str) -> Self {
        let db = Db::open(&path_from_url(url));
        db.call(|conn| {
            let already: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'users'",
                    [],
                    |row| row.get(0),
                )
                .expect("inspect schema");
            if already == 0 {
                conn.execute_batch(SCHEMA).expect("run migrations");
            }
        })
        .await;
        Self { db }
    }
}

impl Backend for DuckDbBackend {
    fn users(&self) -> Arc<dyn UserRepository + Send + Sync> {
        Arc::new(DuckUserRepository::new(self.db.clone()))
    }

    fn sessions(&self) -> Arc<dyn SessionRepository + Send + Sync> {
        Arc::new(DuckSessionRepository::new(self.db.clone()))
    }

    fn sections(&self) -> Arc<dyn SectionRepository + Send + Sync> {
        Arc::new(DuckSectionRepository::new(self.db.clone()))
    }

    fn topics(&self) -> Arc<dyn TopicRepository + Send + Sync> {
        Arc::new(DuckTopicRepository::new(self.db.clone()))
    }

    fn groups(&self) -> Arc<dyn GroupRepository + Send + Sync> {
        Arc::new(DuckGroupRepository::new(self.db.clone()))
    }

    fn comments(&self) -> Arc<dyn CommentRepository + Send + Sync> {
        Arc::new(DuckCommentRepository::new(self.db.clone()))
    }

    fn notifications(&self) -> Arc<dyn NotificationRepository + Send + Sync> {
        Arc::new(DuckNotificationRepository::new(self.db.clone()))
    }

    fn bookmarks(&self) -> Arc<dyn BookmarkRepository + Send + Sync> {
        Arc::new(DuckBookmarkRepository::new(self.db.clone()))
    }

    fn reactions(&self) -> Arc<dyn ReactionRepository + Send + Sync> {
        Arc::new(DuckReactionRepository::new(self.db.clone()))
    }

    fn polls(&self) -> Arc<dyn PollRepository + Send + Sync> {
        Arc::new(DuckPollRepository::new(self.db.clone()))
    }

    fn avatars(&self) -> Arc<dyn AvatarRepository + Send + Sync> {
        Arc::new(DuckAvatarRepository::new(self.db.clone()))
    }

    fn enforcement(&self) -> Arc<dyn EnforcementRepository + Send + Sync> {
        Arc::new(DuckEnforcementRepository::new(self.db.clone()))
    }

    fn abuse(&self) -> Arc<dyn AbuseRepository + Send + Sync> {
        Arc::new(DuckAbuseRepository::new(self.db.clone()))
    }

    fn reports(&self) -> Arc<dyn ReportRepository + Send + Sync> {
        Arc::new(DuckReportRepository::new(self.db.clone()))
    }

    fn watches(&self) -> Arc<dyn WatchRepository + Send + Sync> {
        Arc::new(DuckWatchRepository::new(self.db.clone()))
    }

    fn remarks(&self) -> Arc<dyn RemarkRepository + Send + Sync> {
        Arc::new(DuckRemarkRepository::new(self.db.clone()))
    }

    fn search(&self) -> Arc<dyn SearchRepository + Send + Sync> {
        Arc::new(DuckSearchRepository::new(self.db.clone()))
    }

    fn activity(&self) -> Arc<dyn ActivityRepository + Send + Sync> {
        Arc::new(DuckActivityRepository::new(self.db.clone()))
    }

    fn mail_tokens(&self) -> Arc<dyn MailTokenRepository + Send + Sync> {
        Arc::new(DuckMailTokenRepository::new(self.db.clone()))
    }
}
