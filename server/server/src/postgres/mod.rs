use crate::abuse_repository::PgAbuseRepository;
use crate::activity_repository::PgActivityRepository;
use crate::avatar_repository::PgAvatarRepository;
use crate::backend::Backend;
use crate::bookmark_repository::PgBookmarkRepository;
use crate::comment_repository::PgCommentRepository;
use crate::enforcement_repository::PgEnforcementRepository;
use crate::group_repository::PgGroupRepository;
use crate::mail_token_repository::PgMailTokenRepository;
use crate::notification_repository::PgNotificationRepository;
use crate::poll_repository::PgPollRepository;
use crate::reaction_repository::PgReactionRepository;
use crate::remark_repository::PgRemarkRepository;
use crate::report_repository::PgReportRepository;
use crate::repository::PgUserRepository;
use crate::search_repository::PgSearchRepository;
use crate::section_repository::PgSectionRepository;
use crate::session_repository::PgSessionRepository;
use crate::topic_repository::PgTopicRepository;
use crate::watch_repository::PgWatchRepository;
use app::{
    AbuseRepository, ActivityRepository, AvatarRepository, BookmarkRepository, CommentRepository,
    EnforcementRepository, GroupRepository, InvitationRepository, MailTokenRepository,
    NotificationRepository, PollRepository, ReactionRepository, RemarkRepository, ReportRepository,
    SearchRepository, SectionRepository, SessionRepository, TagRepository, TopicRepository,
    UserRepository, VersionRepository, WatchRepository,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    pub async fn connect(url: &str) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .expect("connect to database");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        Self { pool }
    }
}

impl Backend for PostgresBackend {
    fn users(&self) -> Arc<dyn UserRepository + Send + Sync> {
        Arc::new(PgUserRepository::new(self.pool.clone()))
    }

    fn sessions(&self) -> Arc<dyn SessionRepository + Send + Sync> {
        Arc::new(PgSessionRepository::new(self.pool.clone()))
    }

    fn sections(&self) -> Arc<dyn SectionRepository + Send + Sync> {
        Arc::new(PgSectionRepository::new(self.pool.clone()))
    }

    fn topics(&self) -> Arc<dyn TopicRepository + Send + Sync> {
        Arc::new(PgTopicRepository::new(self.pool.clone()))
    }

    fn groups(&self) -> Arc<dyn GroupRepository + Send + Sync> {
        Arc::new(PgGroupRepository::new(self.pool.clone()))
    }

    fn comments(&self) -> Arc<dyn CommentRepository + Send + Sync> {
        Arc::new(PgCommentRepository::new(self.pool.clone()))
    }

    fn notifications(&self) -> Arc<dyn NotificationRepository + Send + Sync> {
        Arc::new(PgNotificationRepository::new(self.pool.clone()))
    }

    fn bookmarks(&self) -> Arc<dyn BookmarkRepository + Send + Sync> {
        Arc::new(PgBookmarkRepository::new(self.pool.clone()))
    }

    fn reactions(&self) -> Arc<dyn ReactionRepository + Send + Sync> {
        Arc::new(PgReactionRepository::new(self.pool.clone()))
    }

    fn polls(&self) -> Arc<dyn PollRepository + Send + Sync> {
        Arc::new(PgPollRepository::new(self.pool.clone()))
    }

    fn avatars(&self) -> Arc<dyn AvatarRepository + Send + Sync> {
        Arc::new(PgAvatarRepository::new(self.pool.clone()))
    }

    fn enforcement(&self) -> Arc<dyn EnforcementRepository + Send + Sync> {
        Arc::new(PgEnforcementRepository::new(self.pool.clone()))
    }

    fn abuse(&self) -> Arc<dyn AbuseRepository + Send + Sync> {
        Arc::new(PgAbuseRepository::new(self.pool.clone()))
    }

    fn reports(&self) -> Arc<dyn ReportRepository + Send + Sync> {
        Arc::new(PgReportRepository::new(self.pool.clone()))
    }

    fn watches(&self) -> Arc<dyn WatchRepository + Send + Sync> {
        Arc::new(PgWatchRepository::new(self.pool.clone()))
    }

    fn remarks(&self) -> Arc<dyn RemarkRepository + Send + Sync> {
        Arc::new(PgRemarkRepository::new(self.pool.clone()))
    }

    fn invitations(&self) -> Arc<dyn InvitationRepository + Send + Sync> {
        Arc::new(crate::invitation_repository::PgInvitationRepository::new(
            self.pool.clone(),
        ))
    }

    fn versions(&self) -> Arc<dyn VersionRepository + Send + Sync> {
        Arc::new(crate::version_repository::PgVersionRepository::new(
            self.pool.clone(),
        ))
    }

    fn tags(&self) -> Arc<dyn TagRepository + Send + Sync> {
        Arc::new(crate::tag_repository::PgTagRepository::new(
            self.pool.clone(),
        ))
    }

    fn search(&self) -> Arc<dyn SearchRepository + Send + Sync> {
        Arc::new(PgSearchRepository::new(self.pool.clone()))
    }

    fn activity(&self) -> Arc<dyn ActivityRepository + Send + Sync> {
        Arc::new(PgActivityRepository::new(self.pool.clone()))
    }

    fn mail_tokens(&self) -> Arc<dyn MailTokenRepository + Send + Sync> {
        Arc::new(PgMailTokenRepository::new(self.pool.clone()))
    }
}
