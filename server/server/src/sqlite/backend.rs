use crate::backend::Backend;
use crate::sqlite::abuse::SqliteAbuseRepository;
use crate::sqlite::activity::SqliteActivityRepository;
use crate::sqlite::avatar::SqliteAvatarRepository;
use crate::sqlite::bookmark::SqliteBookmarkRepository;
use crate::sqlite::comment::SqliteCommentRepository;
use crate::sqlite::conn::{Db, path_from_url};
use crate::sqlite::enforcement::SqliteEnforcementRepository;
use crate::sqlite::group::SqliteGroupRepository;
use crate::sqlite::invitation::SqliteInvitationRepository;
use crate::sqlite::mail_token::SqliteMailTokenRepository;
use crate::sqlite::notification::SqliteNotificationRepository;
use crate::sqlite::poll::SqlitePollRepository;
use crate::sqlite::reaction::SqliteReactionRepository;
use crate::sqlite::remark::SqliteRemarkRepository;
use crate::sqlite::report::SqliteReportRepository;
use crate::sqlite::search::SqliteSearchRepository;
use crate::sqlite::section::SqliteSectionRepository;
use crate::sqlite::session::SqliteSessionRepository;
use crate::sqlite::topic::SqliteTopicRepository;
use crate::sqlite::user::SqliteUserRepository;
use crate::sqlite::watch::SqliteWatchRepository;
use app::{
    AbuseRepository, ActivityRepository, AttachmentRepository, AvatarRepository,
    BookmarkRepository, CommentRepository, EnforcementRepository, GroupRepository,
    InvitationRepository, MailTokenRepository, NotificationRepository, PollRepository,
    ReactionRepository, RemarkRepository, ReportRepository, SearchRepository, SectionRepository,
    SessionRepository, TagRepository, TopicRepository, UserRepository, VersionRepository,
    WatchRepository,
};
use std::sync::Arc;

pub struct SqliteBackend {
    db: Db,
}

const SCHEMA: &str = include_str!("../../migrations_sqlite/0001_schema.sql");

impl SqliteBackend {
    pub async fn connect(url: &str) -> Self {
        let db = Db::open(&path_from_url(url));
        db.call(|conn| {
            let already: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'users'",
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

impl Backend for SqliteBackend {
    fn users(&self) -> Arc<dyn UserRepository + Send + Sync> {
        Arc::new(SqliteUserRepository::new(self.db.clone()))
    }

    fn sessions(&self) -> Arc<dyn SessionRepository + Send + Sync> {
        Arc::new(SqliteSessionRepository::new(self.db.clone()))
    }

    fn sections(&self) -> Arc<dyn SectionRepository + Send + Sync> {
        Arc::new(SqliteSectionRepository::new(self.db.clone()))
    }

    fn topics(&self) -> Arc<dyn TopicRepository + Send + Sync> {
        Arc::new(SqliteTopicRepository::new(self.db.clone()))
    }

    fn groups(&self) -> Arc<dyn GroupRepository + Send + Sync> {
        Arc::new(SqliteGroupRepository::new(self.db.clone()))
    }

    fn comments(&self) -> Arc<dyn CommentRepository + Send + Sync> {
        Arc::new(SqliteCommentRepository::new(self.db.clone()))
    }

    fn notifications(&self) -> Arc<dyn NotificationRepository + Send + Sync> {
        Arc::new(SqliteNotificationRepository::new(self.db.clone()))
    }

    fn bookmarks(&self) -> Arc<dyn BookmarkRepository + Send + Sync> {
        Arc::new(SqliteBookmarkRepository::new(self.db.clone()))
    }

    fn reactions(&self) -> Arc<dyn ReactionRepository + Send + Sync> {
        Arc::new(SqliteReactionRepository::new(self.db.clone()))
    }

    fn polls(&self) -> Arc<dyn PollRepository + Send + Sync> {
        Arc::new(SqlitePollRepository::new(self.db.clone()))
    }

    fn avatars(&self) -> Arc<dyn AvatarRepository + Send + Sync> {
        Arc::new(SqliteAvatarRepository::new(self.db.clone()))
    }

    fn enforcement(&self) -> Arc<dyn EnforcementRepository + Send + Sync> {
        Arc::new(SqliteEnforcementRepository::new(self.db.clone()))
    }

    fn abuse(&self) -> Arc<dyn AbuseRepository + Send + Sync> {
        Arc::new(SqliteAbuseRepository::new(self.db.clone()))
    }

    fn reports(&self) -> Arc<dyn ReportRepository + Send + Sync> {
        Arc::new(SqliteReportRepository::new(self.db.clone()))
    }

    fn watches(&self) -> Arc<dyn WatchRepository + Send + Sync> {
        Arc::new(SqliteWatchRepository::new(self.db.clone()))
    }

    fn remarks(&self) -> Arc<dyn RemarkRepository + Send + Sync> {
        Arc::new(SqliteRemarkRepository::new(self.db.clone()))
    }

    fn invitations(&self) -> Arc<dyn InvitationRepository + Send + Sync> {
        Arc::new(SqliteInvitationRepository::new(self.db.clone()))
    }

    fn versions(&self) -> Arc<dyn VersionRepository + Send + Sync> {
        Arc::new(crate::sqlite::version::SqliteVersionRepository::new(
            self.db.clone(),
        ))
    }

    fn tags(&self) -> Arc<dyn TagRepository + Send + Sync> {
        Arc::new(crate::sqlite::tag::SqliteTagRepository::new(
            self.db.clone(),
        ))
    }

    fn attachments(&self) -> Arc<dyn AttachmentRepository + Send + Sync> {
        Arc::new(crate::sqlite::attachment::SqliteAttachmentRepository::new(
            self.db.clone(),
        ))
    }

    fn search(&self) -> Arc<dyn SearchRepository + Send + Sync> {
        Arc::new(SqliteSearchRepository::new(self.db.clone()))
    }

    fn activity(&self) -> Arc<dyn ActivityRepository + Send + Sync> {
        Arc::new(SqliteActivityRepository::new(self.db.clone()))
    }

    fn mail_tokens(&self) -> Arc<dyn MailTokenRepository + Send + Sync> {
        Arc::new(SqliteMailTokenRepository::new(self.db.clone()))
    }
}
