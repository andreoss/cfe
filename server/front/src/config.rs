use crate::theme::Theme;
use std::collections::BTreeMap;

pub const BIND_ADDR: &str = "BIND_ADDR";
pub const SERVER_URL: &str = "SERVER_URL";
pub const THEME: &str = "THEME";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: String,
    pub server_url: String,
    pub theme: Theme,
}

impl Config {
    pub fn from_env() -> Self {
        Self::from_vars(&std::env::vars().collect())
    }

    pub fn from_vars(vars: &BTreeMap<String, String>) -> Self {
        Self {
            bind_addr: text(vars, BIND_ADDR, "127.0.0.1:8081"),
            server_url: text(vars, SERVER_URL, "http://127.0.0.1:8080"),
            theme: Theme::parse(text(vars, THEME, Theme::default().name()).as_str())
                .unwrap_or_default(),
        }
    }
}

fn text(vars: &BTreeMap<String, String>, name: &str, fallback: &str) -> String {
    match vars.get(name) {
        Some(value) if !value.trim().is_empty() => value.trim().to_owned(),
        _ => fallback.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn nothing_configured_means_the_documented_defaults() {
        let config = Config::from_vars(&vars(&[]));
        assert_eq!(config.bind_addr, "127.0.0.1:8081");
        assert_eq!(config.server_url, "http://127.0.0.1:8080");
        assert_eq!(config.theme, Theme::Light);
    }

    #[test]
    fn what_is_configured_is_used() {
        let config = Config::from_vars(&vars(&[
            (BIND_ADDR, "0.0.0.0:9000"),
            (SERVER_URL, "http://api:8080"),
            (THEME, "dark"),
        ]));
        assert_eq!(config.bind_addr, "0.0.0.0:9000");
        assert_eq!(config.server_url, "http://api:8080");
        assert_eq!(config.theme, Theme::Dark);
    }

    #[test]
    fn a_theme_nobody_knows_falls_back_to_the_default() {
        let config = Config::from_vars(&vars(&[(THEME, "neon")]));
        assert_eq!(config.theme, Theme::Light);
    }

    #[test]
    fn an_empty_setting_is_the_same_as_no_setting() {
        let config = Config::from_vars(&vars(&[(BIND_ADDR, "  "), (THEME, "")]));
        assert_eq!(config.bind_addr, "127.0.0.1:8081");
        assert_eq!(config.theme, Theme::Light);
    }

    #[test]
    fn surrounding_space_is_trimmed() {
        let config = Config::from_vars(&vars(&[(SERVER_URL, " http://api:1/ ")]));
        assert_eq!(config.server_url, "http://api:1/");
    }
}
