pub mod config;
pub mod html;
pub mod markup;
pub mod routes;
pub mod theme;
pub mod token;

use client::{
    ApiClient, ArchiveMonth, Avatar, Ban, Change, ClientError, Comment, Criteria, Feed, Group, Hit,
    Image, Notification, Paged, Picture, Poll, Profile, Reactions, RegisterBody, Section, Session,
    Subject, Tag, Topic, User, Version, Warning, encode_path, find_section,
};
use config::Config;
use theme::Theme;

pub fn search_address(criteria: &Criteria) -> String {
    format!(
        "/search?q={}&scope={}&order={}",
        encode_path(criteria.query()),
        criteria.scope().label(),
        criteria.order().label()
    )
}

#[derive(Clone)]
pub struct App {
    client: ApiClient,
    default_theme: Theme,
}

impl App {
    pub fn new(config: &Config) -> Result<Self, ClientError> {
        Ok(Self {
            client: ApiClient::new(&config.server_url)?,
            default_theme: config.theme,
        })
    }

    pub fn default_theme(&self) -> Theme {
        self.default_theme
    }

    pub async fn sections(&self) -> Result<Vec<Section>, ClientError> {
        self.client.sections().await
    }

    pub async fn section(&self, slug: &str) -> Result<Option<Section>, ClientError> {
        Ok(find_section(&self.sections().await?, slug).cloned())
    }

    pub async fn topics(&self, slug: &str, page: u32) -> Result<Paged<Topic>, ClientError> {
        self.client.topics(slug, page).await
    }

    pub async fn subject(
        &self,
        session: Option<&str>,
        id: &str,
    ) -> Result<Option<Subject>, ClientError> {
        match self.client.subject(id, session).await {
            Ok(subject) => Ok(Some(subject)),
            Err(error) if error.status() == Some(404) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn comments(
        &self,
        session: Option<&str>,
        id: &str,
        page: u32,
    ) -> Result<Paged<Comment>, ClientError> {
        self.client.comments(id, page, session).await
    }

    pub async fn create_topic(
        &self,
        session: &str,
        slug: &str,
        body: &client::NewSubject,
    ) -> Result<Subject, ClientError> {
        self.client.create_topic(session, slug, body).await
    }

    pub async fn create_comment(
        &self,
        session: &str,
        topic_id: &str,
        body: &client::NewRemark,
    ) -> Result<Comment, ClientError> {
        self.client.create_comment(session, topic_id, body).await
    }

    pub async fn edit_topic(
        &self,
        session: &str,
        id: &str,
        body: &client::SubjectEdit,
    ) -> Result<Subject, ClientError> {
        self.client.edit_topic(session, id, body).await
    }

    pub async fn edit_comment(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
        body: &str,
    ) -> Result<Comment, ClientError> {
        self.client.edit_comment(session, topic_id, id, body).await
    }

    pub async fn delete_topic(
        &self,
        session: &str,
        id: &str,
        body: &client::Removal,
    ) -> Result<Subject, ClientError> {
        self.client.delete_topic(session, id, body).await
    }

    pub async fn restore_topic(&self, session: &str, id: &str) -> Result<Subject, ClientError> {
        self.client.restore_topic(session, id).await
    }

    pub async fn delete_comment(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
        body: &client::Removal,
    ) -> Result<Comment, ClientError> {
        self.client
            .delete_comment(session, topic_id, id, body)
            .await
    }

    pub async fn restore_comment(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
    ) -> Result<Comment, ClientError> {
        self.client.restore_comment(session, topic_id, id).await
    }

    pub async fn topic_history(&self, id: &str) -> Result<Vec<Version>, ClientError> {
        self.client.topic_history(id).await
    }

    pub async fn topic_difference(
        &self,
        id: &str,
        version_id: &str,
    ) -> Result<Vec<Change>, ClientError> {
        self.client.topic_difference(id, version_id).await
    }

    pub async fn publish(&self, session: &str, id: &str) -> Result<Subject, ClientError> {
        self.client.publish(session, id).await
    }

    pub async fn commit(&self, session: &str, id: &str) -> Result<Subject, ClientError> {
        self.client.commit(session, id).await
    }

    pub async fn uncommit(&self, session: &str, id: &str) -> Result<Subject, ClientError> {
        self.client.uncommit(session, id).await
    }

    pub async fn sticky(
        &self,
        session: &str,
        id: &str,
        pinned: bool,
    ) -> Result<Subject, ClientError> {
        self.client.sticky(session, id, pinned).await
    }

    pub async fn off_front(
        &self,
        session: &str,
        id: &str,
        hidden: bool,
    ) -> Result<Subject, ClientError> {
        self.client.off_front(session, id, hidden).await
    }

    pub async fn resolved(
        &self,
        session: &str,
        id: &str,
        done: bool,
    ) -> Result<Subject, ClientError> {
        self.client.resolved(session, id, done).await
    }

    pub async fn postscore(
        &self,
        session: &str,
        id: &str,
        score: i32,
    ) -> Result<Subject, ClientError> {
        self.client.postscore(session, id, score).await
    }

    pub async fn move_to(
        &self,
        session: &str,
        id: &str,
        group: &str,
    ) -> Result<Subject, ClientError> {
        self.client.move_to(session, id, group).await
    }

    pub async fn groups(&self, slug: &str) -> Result<Vec<Group>, ClientError> {
        self.client.groups(slug).await
    }

    pub async fn images(&self, id: &str) -> Result<Vec<Image>, ClientError> {
        self.client.images(id).await
    }

    pub async fn image(
        &self,
        topic_id: &str,
        image_id: &str,
    ) -> Result<Option<Picture>, ClientError> {
        self.client.image(topic_id, image_id).await
    }

    pub async fn attach_image(
        &self,
        session: &str,
        id: &str,
        data: &str,
    ) -> Result<Image, ClientError> {
        self.client.attach_image(session, id, data).await
    }

    pub async fn remove_image(
        &self,
        session: &str,
        topic_id: &str,
        image_id: &str,
    ) -> Result<(), ClientError> {
        self.client.remove_image(session, topic_id, image_id).await
    }

    pub async fn poll(&self, session: Option<&str>, id: &str) -> Result<Option<Poll>, ClientError> {
        self.client.poll(session, id).await
    }

    pub async fn create_poll(
        &self,
        session: &str,
        id: &str,
        question: &str,
        options: &[&str],
    ) -> Result<Poll, ClientError> {
        self.client
            .create_poll(session, id, question, options)
            .await
    }

    pub async fn vote(
        &self,
        session: &str,
        id: &str,
        option_id: &str,
    ) -> Result<Poll, ClientError> {
        self.client.vote(session, id, option_id).await
    }

    pub async fn topic_reactions(
        &self,
        session: Option<&str>,
        id: &str,
    ) -> Result<Reactions, ClientError> {
        self.client.topic_reactions(session, id).await
    }

    pub async fn comment_reactions(
        &self,
        session: Option<&str>,
        topic_id: &str,
        id: &str,
    ) -> Result<Reactions, ClientError> {
        self.client.comment_reactions(session, topic_id, id).await
    }

    pub async fn react_to_topic(
        &self,
        session: &str,
        id: &str,
        kind: &str,
    ) -> Result<Reactions, ClientError> {
        self.client.react_to_topic(session, id, kind).await
    }

    pub async fn react_to_comment(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
        kind: &str,
    ) -> Result<Reactions, ClientError> {
        self.client
            .react_to_comment(session, topic_id, id, kind)
            .await
    }

    pub async fn clear_topic_reaction(
        &self,
        session: &str,
        id: &str,
    ) -> Result<Reactions, ClientError> {
        self.client.clear_topic_reaction(session, id).await
    }

    pub async fn clear_comment_reaction(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
    ) -> Result<Reactions, ClientError> {
        self.client
            .clear_comment_reaction(session, topic_id, id)
            .await
    }

    pub async fn search(&self, criteria: &Criteria) -> Result<Vec<Hit>, ClientError> {
        self.client.search(criteria).await
    }

    pub async fn archive(&self, session: Option<&str>) -> Result<Vec<ArchiveMonth>, ClientError> {
        self.client.archive(session).await
    }

    pub async fn archive_month(
        &self,
        year: i32,
        month: u8,
        page: u32,
        session: Option<&str>,
    ) -> Result<Option<Paged<Topic>>, ClientError> {
        match self.client.archive_month(year, month, page, session).await {
            Ok(topics) => Ok(Some(topics)),
            Err(error) if matches!(error.status(), Some(404 | 422)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn bookmarks(&self, session: &str, page: u32) -> Result<Paged<Topic>, ClientError> {
        self.client.bookmarks(session, page).await
    }

    pub async fn watched(&self, session: &str, page: u32) -> Result<Paged<Topic>, ClientError> {
        self.client.watched(session, page).await
    }

    pub async fn followed_tags(&self, session: &str) -> Result<Vec<String>, ClientError> {
        self.client.followed_tags(session).await
    }

    pub async fn notifications(
        &self,
        session: &str,
        page: u32,
    ) -> Result<Paged<Notification>, ClientError> {
        self.client.notifications(session, page).await
    }

    pub async fn mark_read(&self, session: &str, id: &str) -> Result<(), ClientError> {
        self.client.mark_read(session, id).await
    }

    pub async fn activity(&self, session: Option<&str>) -> Result<Vec<Hit>, ClientError> {
        self.client.activity(session).await
    }

    pub async fn tag(&self, name: &str, session: Option<&str>) -> Result<Tag, ClientError> {
        self.client.tag(name, session).await
    }

    pub async fn tag_topics(
        &self,
        name: &str,
        page: u32,
        session: Option<&str>,
    ) -> Result<Paged<Topic>, ClientError> {
        self.client.tag_topics(name, page, session).await
    }

    pub async fn follow_tag(&self, session: &str, name: &str) -> Result<(), ClientError> {
        self.client.follow_tag(session, name).await
    }

    pub async fn unfollow_tag(&self, session: &str, name: &str) -> Result<(), ClientError> {
        self.client.unfollow_tag(session, name).await
    }

    pub async fn describe_tag(
        &self,
        session: &str,
        name: &str,
        words: Option<&str>,
        means: Option<&str>,
    ) -> Result<(), ClientError> {
        if let Some(words) = words {
            self.client.describe_tag(session, name, words).await?;
        }
        if let Some(means) = means {
            self.client.say_means(session, name, means).await?;
        }
        Ok(())
    }

    pub async fn section_feed(&self, slug: &str) -> Result<Option<Feed>, ClientError> {
        self.client.section_feed(slug).await
    }

    pub async fn tag_feed(&self, name: &str) -> Result<Option<Feed>, ClientError> {
        self.client.tag_feed(name).await
    }

    pub async fn account(&self, session: &str) -> Option<User> {
        self.client.me(Some(session)).await.ok().flatten()
    }

    pub async fn profile(&self, username: &str) -> Result<Option<Profile>, ClientError> {
        match self.client.profile(username).await {
            Ok(profile) => Ok(Some(profile)),
            Err(error) if error.status() == Some(404) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn avatar(&self, username: &str) -> Option<Avatar> {
        self.client.avatar(username).await.ok().flatten()
    }

    pub async fn update_bio(
        &self,
        session: &str,
        bio: Option<&str>,
    ) -> Result<Profile, ClientError> {
        self.client.update_bio(session, bio).await
    }

    pub async fn warnings(&self, session: &str) -> Result<Vec<Warning>, ClientError> {
        self.client.warnings(session).await
    }

    pub async fn acknowledge_warnings(&self, session: &str) -> Result<(), ClientError> {
        self.client.acknowledge_warnings(session).await
    }

    pub async fn ignore_state(&self, session: &str, username: &str) -> Option<bool> {
        self.client.ignore_state(session, username).await.ok()
    }

    pub async fn ban_state(&self, session: &str, username: &str) -> Option<Ban> {
        self.client
            .ban_state(session, username)
            .await
            .ok()
            .flatten()
    }

    pub async fn ban(
        &self,
        session: &str,
        username: &str,
        reason: &str,
        days: Option<u32>,
    ) -> Result<(), ClientError> {
        self.client.ban(session, username, reason, days).await?;
        Ok(())
    }

    pub async fn lift_ban(&self, session: &str, username: &str) -> Result<(), ClientError> {
        self.client.lift_ban(session, username).await
    }

    pub async fn warn(
        &self,
        session: &str,
        username: &str,
        reason: &str,
    ) -> Result<(), ClientError> {
        self.client.warn(session, username, reason).await?;
        Ok(())
    }

    pub async fn promote(&self, session: &str, username: &str) -> Result<(), ClientError> {
        self.client.promote(session, username).await?;
        Ok(())
    }

    pub async fn set_role(
        &self,
        session: &str,
        username: &str,
        role: &str,
    ) -> Result<(), ClientError> {
        self.client.set_role(session, username, role).await?;
        Ok(())
    }

    pub async fn ignore(&self, session: &str, username: &str) -> Result<(), ClientError> {
        self.client.ignore(session, username).await
    }

    pub async fn stop_ignoring(&self, session: &str, username: &str) -> Result<(), ClientError> {
        self.client.stop_ignoring(session, username).await
    }

    pub async fn remark(&self, session: &str, username: &str) -> Option<String> {
        self.client.remark(session, username).await.ok().flatten()
    }

    pub async fn set_remark(
        &self,
        session: &str,
        username: &str,
        text: &str,
    ) -> Result<(), ClientError> {
        self.client.set_remark(session, username, text).await?;
        Ok(())
    }

    pub async fn clear_remark(&self, session: &str, username: &str) -> Result<(), ClientError> {
        self.client.clear_remark(session, username).await
    }

    pub async fn register(&self, body: &RegisterBody) -> Result<Session, ClientError> {
        self.client.register(body).await
    }

    pub async fn sign_in(&self, username: &str, password: &str) -> Result<Session, ClientError> {
        self.client.sign_in(username, password).await
    }

    pub async fn sign_out(&self, session: &str) -> Option<String> {
        self.client.sign_out(session).await.ok().flatten()
    }

    pub async fn change_password(
        &self,
        session: &str,
        current: &str,
        new: &str,
    ) -> Result<Option<String>, ClientError> {
        self.client.change_password(session, current, new).await
    }

    pub async fn request_reset(&self, email: &str) -> Result<(), ClientError> {
        self.client.request_reset(email).await
    }

    pub async fn reset_password(&self, code: &str, new: &str) -> Result<(), ClientError> {
        self.client.reset_password(code, new).await
    }

    pub async fn request_email_change(
        &self,
        session: &str,
        email: &str,
    ) -> Result<(), ClientError> {
        self.client.request_email_change(session, email).await
    }

    pub async fn confirm_email(&self, code: &str) -> Result<User, ClientError> {
        self.client.confirm_email(code).await
    }

    pub async fn activate(&self, code: &str) -> Result<User, ClientError> {
        self.client.activate(code).await
    }

    pub async fn deregister(&self, session: &str) -> Result<Option<String>, ClientError> {
        self.client.deregister(session).await
    }

    pub fn theme_for(&self, cookie: Option<&str>) -> Theme {
        cookie.and_then(Theme::parse).unwrap_or(self.default_theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::from_vars(&std::collections::BTreeMap::new())
    }

    #[test]
    fn an_app_is_built_from_a_configuration() {
        let app = App::new(&config()).unwrap();
        assert_eq!(app.default_theme(), Theme::Light);
    }

    #[test]
    fn an_app_refuses_a_configuration_that_is_not_an_address() {
        let mut vars = std::collections::BTreeMap::new();
        vars.insert(
            config::SERVER_URL.to_owned(),
            "postgres://localhost/tcbs".to_owned(),
        );
        let config = Config::from_vars(&vars);
        assert!(App::new(&config).is_err());
    }

    #[test]
    fn a_cookie_chooses_the_theme_and_an_unreadable_one_does_not() {
        let app = App::new(&config()).unwrap();
        assert_eq!(app.theme_for(Some("dark")), Theme::Dark);
        assert_eq!(app.theme_for(Some("neon")), Theme::Light);
        assert_eq!(app.theme_for(None), Theme::Light);
    }
}
