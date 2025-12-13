pub mod config;
pub mod html;
pub mod routes;
pub mod theme;

use client::{ApiClient, ClientError, Section};
use config::Config;
use theme::Theme;

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
}
