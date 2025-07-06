use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};

/// テーマ用のスタイル
/// - 上位レイヤーで指定されていない場合は下位レイヤーのスタイルを継承する
/// - 上位レイヤーで指定されている場合は上位レイヤーのスタイルを優先する
/// - Modifierは加算方式
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct ThemeStyleConfig {
    #[serde(with = "serde_color", skip_serializing_if = "Option::is_none", default)]
    pub fg_color: Option<Color>,

    #[serde(with = "serde_color", skip_serializing_if = "Option::is_none", default)]
    pub bg_color: Option<Color>,

    #[serde(
        with = "serde_modifier",
        skip_serializing_if = "Modifier::is_empty",
        default
    )]
    pub modifier: Modifier,
}

impl From<ThemeStyleConfig> for ratatui::style::Style {
    fn from(config: ThemeStyleConfig) -> Self {
        let mut style = ratatui::style::Style::new();

        if let Some(fg_color) = config.fg_color {
            style = style.fg(fg_color);
        }

        if let Some(bg_color) = config.bg_color {
            style = style.bg(bg_color);
        }

        if !config.modifier.is_empty() {
            style = style.add_modifier(config.modifier);
        }

        style
    }
}

/// Modifierに対して大文字・小文字を区別せずにパースできるように拡張する
mod serde_modifier {
    use serde::Deserialize as _;

    use super::Modifier;

    const DEFAULT_MODIFIER_NAME: &str = "NONE";

    pub fn serialize<S>(modifier: &Modifier, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if modifier.is_empty() {
            return serializer.serialize_str(&DEFAULT_MODIFIER_NAME.to_lowercase());
        }

        let lowercase_str = format!("{modifier:?}").to_lowercase();

        serializer.serialize_str(&lowercase_str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Modifier, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let uppercase_str = s.to_uppercase();

        match uppercase_str.as_str() {
            DEFAULT_MODIFIER_NAME => Ok(Modifier::empty()),
            _ => bitflags::parser::from_str::<Modifier>(&uppercase_str)
                .map_err(serde::de::Error::custom),
        }
    }
}

mod serde_color {
    use std::str::FromStr;

    use serde::Deserialize as _;

    use super::Color;

    const DEFAULT_COLOR_NAME: &str = "default";

    /// Colorをシリアライズした結果を小文字に変換する
    pub fn serialize<S>(color: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match color {
            Some(c) if *c != Color::default() => {
                serializer.serialize_str(&c.to_string().to_lowercase())
            }
            None => serializer.serialize_none(),
            _ => serializer.serialize_str(DEFAULT_COLOR_NAME),
        }
    }

    /// Colorをデシリアライズする際に"default"文字列をサポートし、Color::Defaultに変換する
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        match s.as_str() {
            DEFAULT_COLOR_NAME => Ok(Some(Color::default())),
            _ => Ok(Some(Color::from_str(&s).map_err(serde::de::Error::custom)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[test]
    fn default_theme_style_config() {
        let actual = ThemeStyleConfig::default();

        let expected = ThemeStyleConfig {
            fg_color: None,
            bg_color: None,
            modifier: Modifier::empty(),
        };

        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case::default_theme_style_config(ThemeStyleConfig::default(), "{}")]
    #[case::with_default_fg_color(
        ThemeStyleConfig {
            fg_color: Some(Color::default()),
            ..Default::default()
        },
        "fg_color: default"
    )]
    #[case::with_fg_color(
        ThemeStyleConfig {
            fg_color: Some(Color::Red),
            ..Default::default()
        },
        "fg_color: red"
    )]
    #[case::with_default_bg_color(
        ThemeStyleConfig {
            bg_color: Some(Color::default()),
            ..Default::default()
        },
        "bg_color: default"
    )]
    #[case::with_bg_color(
        ThemeStyleConfig {
            bg_color: Some(Color::Blue),
            ..Default::default()
        },
        "bg_color: blue"
    )]
    #[case::with_modifier(
        ThemeStyleConfig {
            modifier: Modifier::BOLD,
            ..Default::default()
        },
        "modifier: bold"
    )]
    #[case::with_multiple_modifiers(
        ThemeStyleConfig {
            modifier: Modifier::BOLD | Modifier::ITALIC,
            ..Default::default()
        },
        "modifier: bold | italic"
    )]
    #[case::with_none_modifier(
        ThemeStyleConfig {
            modifier: Modifier::empty(),
            ..Default::default()
        },
        "{}"
    )]
    #[case::combined_styles(
        ThemeStyleConfig {
            fg_color: Some(Color::Red),
            bg_color: Some(Color::Blue),
            modifier: Modifier::BOLD,
        },
        indoc! {"
            fg_color: red
            bg_color: blue
            modifier: bold
        "}
    )]
    fn test_serialize_theme_style_config(#[case] actual: ThemeStyleConfig, #[case] expected: &str) {
        let serialized = serde_yaml::to_string(&actual).unwrap();
        assert_eq!(serialized.trim(), expected.trim());
    }

    #[rstest]
    #[case::default_theme_style_config("", ThemeStyleConfig::default())]
    #[case::with_default_fg_color(
        "fg_color: default",
        ThemeStyleConfig {
            fg_color: Some(Color::default()),
            ..Default::default()
        }
    )]
    #[case::with_fg_color(
        "fg_color: red",
        ThemeStyleConfig {
            fg_color: Some(Color::Red),
            ..Default::default()
        }
    )]
    #[case::with_default_bg_color(
        "bg_color: default",
        ThemeStyleConfig {
            bg_color: Some(Color::default()),
            ..Default::default()
        }
    )]
    #[case::with_bg_color(
        "bg_color: blue",
        ThemeStyleConfig {
            bg_color: Some(Color::Blue),
            ..Default::default()
        }
    )]
    #[case::with_modifier(
        "modifier: bold",
        ThemeStyleConfig {
            modifier: Modifier::BOLD,
            ..Default::default()
        }
    )]
    #[case::with_multiple_modifiers(
        "modifier: bold | italic",
        ThemeStyleConfig {
            modifier: Modifier::BOLD | Modifier::ITALIC,
            ..Default::default()
        }
    )]
    #[case::with_none_modifier(
        "modifier: none",
        ThemeStyleConfig {
            modifier: Modifier::empty(),
            ..Default::default()
        }
    )]
    #[case::combined_styles(
        indoc! {"
            fg_color: red
            bg_color: blue
            modifier: bold
        "},
        ThemeStyleConfig {
            fg_color: Some(Color::Red),
            bg_color: Some(Color::Blue),
            modifier: Modifier::BOLD,
        }
    )]
    fn test_deserialize_theme_style_config(
        #[case] actual: &str,
        #[case] expected: ThemeStyleConfig,
    ) {
        let deserialized: ThemeStyleConfig = serde_yaml::from_str(actual).unwrap();
        assert_eq!(deserialized, expected);
    }
}
