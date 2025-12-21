pub mod config;
pub mod html;
pub mod markup;
pub mod routes;
pub mod theme;
pub mod token;

use client::{
    ApiClient, ArchiveMonth, Avatar, ClientError, Comment, Criteria, Feed, Hit, Paged, Profile,
    RegisterBody, Section, Session, Subject, Tag, Topic, User, encode_path, find_section,
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

    pub async fn subject(&self, id: &str) -> Result<Option<Subject>, ClientError> {
        match self.client.subject(id).await {
            Ok(subject) => Ok(Some(subject)),
            Err(error) if error.status() == Some(404) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn comments(&self, id: &str, page: u32) -> Result<Paged<Comment>, ClientError> {
        self.client.comments(id, page).await
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
