use ratatui::{style::Color, widgets::BorderType};
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

/// フォーカスイベントありのスタイル
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BorderThemeConfig {
    #[serde(default, rename = "type", with = "serde_border_type")]
    pub ty: BorderType,

    #[serde(default)]
    pub active: ThemeStyleConfig,

    #[serde(default = "default_inactive")]
    pub inactive: ThemeStyleConfig,

    #[serde(default = "default_mouse_over")]
    pub mouse_over: ThemeStyleConfig,
}

impl Default for BorderThemeConfig {
    fn default() -> Self {
        Self {
            ty: BorderType::default(),
            active: ThemeStyleConfig::default(),
            inactive: default_inactive(),
            mouse_over: default_mouse_over(),
        }
    }
}

/// BorderThemeのinactiveのデフォルト値となるThemeStyleを返す
fn default_inactive() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::DarkGray),
        ..Default::default()
    }
}

fn default_mouse_over() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::Gray),
        ..Default::default()
    }
}

mod serde_border_type {
    use std::str::FromStr;

    use serde::Deserialize as _;

    use super::BorderType;

    pub fn serialize<S>(ty: &BorderType, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let lowercase_str = ty.to_string().to_lowercase();

        serializer.serialize_str(&lowercase_str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BorderType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let capitalized = capitalize(&s);

        BorderType::from_str(&capitalized).map_err(serde::de::Error::custom)
    }

    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().chain(c).collect(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use pretty_assertions::assert_eq;
        use rstest::rstest;

        #[rstest]
        #[case("a", "A")]
        #[case("A", "A")]
        fn capitalize_single_char(#[case] actual: &str, #[case] expected: &str) {
            assert_eq!(capitalize(actual), expected);
        }

        #[rstest]
        #[case("hello", "Hello")]
        #[case("Hello", "Hello")]
        #[case("hELLO", "HELLO")]
        fn capitalize_multiple_chars(#[case] actual: &str, #[case] expected: &str) {
            assert_eq!(capitalize(actual), expected);
        }

        #[rstest]
        #[case("1hello", "1hello")]
        #[case("!hello", "!hello")]
        fn capitalize_non_alphabetic(#[case] actual: &str, #[case] expected: &str) {
            assert_eq!(capitalize(actual), expected);
        }
    }
}
