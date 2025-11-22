use crate::abuse_repository::PgAbuseRepository;
use crate::activity_repository::PgActivityRepository;
use crate::avatar_repository::PgAvatarRepository;
use crate::backend::Backend;
use crate::bookmark_repository::PgBookmarkRepository;
use crate::comment_repository::PgCommentRepository;
use crate::enforcement_repository::PgEnforcementRepository;
use crate::mail_token_repository::PgMailTokenRepository;
use crate::notification_repository::PgNotificationRepository;
use crate::poll_repository::PgPollRepository;
use crate::reaction_repository::PgReactionRepository;
use crate::repository::PgUserRepository;
use crate::search_repository::PgSearchRepository;
use crate::section_repository::PgSectionRepository;
use crate::session_repository::PgSessionRepository;
use crate::topic_repository::PgTopicRepository;
use app::{
    AbuseRepository, ActivityRepository, AvatarRepository, BookmarkRepository, CommentRepository,
    EnforcementRepository, MailTokenRepository, NotificationRepository, PollRepository,
    ReactionRepository, SearchRepository, SectionRepository, SessionRepository, TopicRepository,
    UserRepository,
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
