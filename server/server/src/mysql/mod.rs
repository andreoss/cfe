mod abuse;
mod activity;
mod avatar;
mod bookmark;
mod comment;
mod enforcement;
mod group;
mod mail_token;
mod notification;
mod poll;
mod reaction;
mod report;
mod search;
mod section;
mod session;
mod topic;
mod user;
mod watch;

use crate::backend::Backend;
use app::{
    AbuseRepository, ActivityRepository, AvatarRepository, BookmarkRepository, CommentRepository,
    EnforcementRepository, GroupRepository, MailTokenRepository, NotificationRepository,
    PollRepository, ReactionRepository, ReportRepository, SearchRepository, SectionRepository,
    SessionRepository, TopicRepository, UserRepository, WatchRepository,
};
use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;
use std::sync::Arc;

pub struct MySqlBackend {
    pool: MySqlPool,
}

impl MySqlBackend {
    pub async fn connect(url: &str) -> Self {
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .expect("connect to database");
        sqlx::migrate!("./migrations_mysql")
            .run(&pool)
            .await
            .expect("run migrations");
        Self { pool }
    }
}

impl Backend for MySqlBackend {
    fn users(&self) -> Arc<dyn UserRepository + Send + Sync> {
        Arc::new(user::MySqlUserRepository::new(self.pool.clone()))
    }

    fn sessions(&self) -> Arc<dyn SessionRepository + Send + Sync> {
        Arc::new(session::MySqlSessionRepository::new(self.pool.clone()))
    }

    fn sections(&self) -> Arc<dyn SectionRepository + Send + Sync> {
        Arc::new(section::MySqlSectionRepository::new(self.pool.clone()))
    }

    fn topics(&self) -> Arc<dyn TopicRepository + Send + Sync> {
        Arc::new(topic::MySqlTopicRepository::new(self.pool.clone()))
    }

    fn groups(&self) -> Arc<dyn GroupRepository + Send + Sync> {
        Arc::new(group::MySqlGroupRepository::new(self.pool.clone()))
    }

    fn comments(&self) -> Arc<dyn CommentRepository + Send + Sync> {
        Arc::new(comment::MySqlCommentRepository::new(self.pool.clone()))
    }

    fn notifications(&self) -> Arc<dyn NotificationRepository + Send + Sync> {
        Arc::new(notification::MySqlNotificationRepository::new(
            self.pool.clone(),
        ))
    }

    fn bookmarks(&self) -> Arc<dyn BookmarkRepository + Send + Sync> {
        Arc::new(bookmark::MySqlBookmarkRepository::new(self.pool.clone()))
    }

    fn reactions(&self) -> Arc<dyn ReactionRepository + Send + Sync> {
        Arc::new(reaction::MySqlReactionRepository::new(self.pool.clone()))
    }

    fn polls(&self) -> Arc<dyn PollRepository + Send + Sync> {
        Arc::new(poll::MySqlPollRepository::new(self.pool.clone()))
    }

    fn avatars(&self) -> Arc<dyn AvatarRepository + Send + Sync> {
        Arc::new(avatar::MySqlAvatarRepository::new(self.pool.clone()))
    }

    fn enforcement(&self) -> Arc<dyn EnforcementRepository + Send + Sync> {
        Arc::new(enforcement::MySqlEnforcementRepository::new(
            self.pool.clone(),
        ))
    }

    fn abuse(&self) -> Arc<dyn AbuseRepository + Send + Sync> {
        Arc::new(abuse::MySqlAbuseRepository::new(self.pool.clone()))
    }

    fn reports(&self) -> Arc<dyn ReportRepository + Send + Sync> {
        Arc::new(report::MySqlReportRepository::new(self.pool.clone()))
    }

    fn watches(&self) -> Arc<dyn WatchRepository + Send + Sync> {
        Arc::new(watch::MySqlWatchRepository::new(self.pool.clone()))
    }

    fn search(&self) -> Arc<dyn SearchRepository + Send + Sync> {
        Arc::new(search::MySqlSearchRepository::new(self.pool.clone()))
    }

    fn activity(&self) -> Arc<dyn ActivityRepository + Send + Sync> {
        Arc::new(activity::MySqlActivityRepository::new(self.pool.clone()))
    }

    fn mail_tokens(&self) -> Arc<dyn MailTokenRepository + Send + Sync> {
        Arc::new(mail_token::MySqlMailTokenRepository::new(self.pool.clone()))
    }
}
