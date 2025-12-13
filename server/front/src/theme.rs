#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Light,
    Classic,
    Dark,
    Contrast,
}

impl Theme {
    pub const ALL: [Theme; 4] = [Theme::Light, Theme::Classic, Theme::Dark, Theme::Contrast];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "light" => Some(Theme::Light),
            "classic" => Some(Theme::Classic),
            "dark" => Some(Theme::Dark),
            "contrast" => Some(Theme::Contrast),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Classic => "classic",
            Theme::Dark => "dark",
            Theme::Contrast => "contrast",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Classic => "Classic",
            Theme::Dark => "Dark",
            Theme::Contrast => "High contrast",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_parses_from_its_own_name() {
        for theme in Theme::ALL {
            assert_eq!(Theme::parse(theme.name()), Some(theme));
        }
    }

    #[test]
    fn parsing_ignores_case_and_surrounding_space() {
        assert_eq!(Theme::parse(" Dark "), Some(Theme::Dark));
        assert_eq!(Theme::parse("HIGH-CONTRAST"), None);
    }

    #[test]
    fn an_unknown_name_is_not_a_theme() {
        assert_eq!(Theme::parse("neon"), None);
        assert_eq!(Theme::parse(""), None);
    }

    #[test]
    fn the_default_theme_is_the_light_one() {
        assert_eq!(Theme::default(), Theme::Light);
    }

    #[test]
    fn every_theme_has_a_label_a_reader_can_choose_by() {
        for theme in Theme::ALL {
            assert!(!theme.label().is_empty());
        }
        assert_eq!(Theme::Contrast.label(), "High contrast");
    }
}
