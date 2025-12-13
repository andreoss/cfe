pub mod config;
pub mod routes;
pub mod theme;

use config::Config;
use theme::Theme;

#[derive(Clone)]
pub struct App {
    default_theme: Theme,
}

impl App {
    pub fn new(config: &Config) -> Self {
        Self {
            default_theme: config.theme,
        }
    }

    pub fn default_theme(&self) -> Theme {
        self.default_theme
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
        let app = App::new(&config());
        assert_eq!(app.default_theme(), Theme::Light);
    }
}
