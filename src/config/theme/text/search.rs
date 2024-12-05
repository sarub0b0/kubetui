use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};

use crate::{
    config::theme::ThemeStyleConfig,
    ui::widget::{
        InputFormTheme, SearchHighlightFocusStyle, SearchHighlightMatchesStyle,
        SearchHighlightStyle,
    },
};

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SearchThemeConfig {
    #[serde(default)]
    pub form: SearchFormThemeConfig,

    #[serde(default)]
    pub highlight: SearchHighlightThemeConfig,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SearchFormThemeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<ThemeStyleConfig>,

    #[serde(default)]
    pub prefix: ThemeStyleConfig,

    #[serde(default)]
    pub query: ThemeStyleConfig,

    #[serde(default)]
    pub suffix: ThemeStyleConfig,
}

impl From<SearchFormThemeConfig> for InputFormTheme {
    fn from(theme: SearchFormThemeConfig) -> Self {
        InputFormTheme::default()
            .prefix_style(theme.prefix)
            .content_style(theme.query)
            .suffix_style(theme.suffix)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SearchHighlightThemeConfig {
    #[serde(default = "default_search_highlight_focus")]
    pub focus: ThemeStyleConfig,

    #[serde(default = "default_search_highlight_matches")]
    pub matches: ThemeStyleConfig,
}

impl Default for SearchHighlightThemeConfig {
    fn default() -> Self {
        Self {
            focus: default_search_highlight_focus(),
            matches: default_search_highlight_matches(),
        }
    }
}

fn default_search_highlight_focus() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::Yellow),
        modifier: Modifier::REVERSED,
        ..Default::default()
    }
}

fn default_search_highlight_matches() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::REVERSED,
        ..Default::default()
    }
}

impl From<SearchHighlightThemeConfig> for SearchHighlightStyle {
    fn from(value: SearchHighlightThemeConfig) -> Self {
        Self {
            focus: SearchHighlightFocusStyle::new(value.focus),
            matches: SearchHighlightMatchesStyle::new(value.matches),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml;

    mod form {
        use super::*;
        use indoc::indoc;

        #[test]
        fn default() {
            let default_config = SearchFormThemeConfig::default();
            assert_eq!(default_config.base, None);
            assert_eq!(default_config.prefix, ThemeStyleConfig::default());
            assert_eq!(default_config.query, ThemeStyleConfig::default());
            assert_eq!(default_config.suffix, ThemeStyleConfig::default());
        }

        #[test]
        fn serialize() {
            let config = SearchFormThemeConfig {
                base: Some(ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                }),
                prefix: ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                },
                query: ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                },
                suffix: ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                },
            };

            let actual = serde_yaml::to_string(&config).unwrap();

            let expected = indoc! {"
                base:
                  fg_color: yellow
                prefix:
                  fg_color: yellow
                query:
                  fg_color: yellow
                suffix:
                  fg_color: yellow
            "};

            assert_eq!(actual, expected);
        }

        #[test]
        fn deserialize() {
            let yaml_data = indoc! {"
                base:
                  fg_color: yellow
                prefix:
                  fg_color: yellow
                query:
                  fg_color: yellow
                suffix:
                  fg_color: yellow
            "};

            let actual: SearchFormThemeConfig = serde_yaml::from_str(yaml_data).unwrap();

            let expected = SearchFormThemeConfig {
                base: Some(ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                }),
                prefix: ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                },
                query: ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                },
                suffix: ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                },
            };

            assert_eq!(actual, expected);
        }

        #[test]
        fn skip_serializing_if_none() {
            let config = SearchFormThemeConfig {
                base: None,
                prefix: ThemeStyleConfig::default(),
                query: ThemeStyleConfig::default(),
                suffix: ThemeStyleConfig::default(),
            };

            let serialized = serde_yaml::to_string(&config).unwrap();
            let expected = indoc! {"
                prefix: {}
                query: {}
                suffix: {}
            "};

            assert_eq!(serialized.trim(), expected.trim());
        }
    }

    mod highlight {
        use super::*;
        use indoc::indoc;

        #[test]
        fn default() {
            let default_config = SearchHighlightThemeConfig::default();

            assert_eq!(default_config.focus.fg_color, Some(Color::Yellow));
            assert_eq!(default_config.focus.modifier, Modifier::REVERSED);
            assert_eq!(default_config.matches.modifier, Modifier::REVERSED);
        }

        #[test]
        fn deserialize() {
            let yaml_data = indoc! {r#"
                focus:
                  fg_color: Yellow
                  modifier: REVERSED
                matches:
                  modifier: REVERSED
            "# };

            let config: SearchHighlightThemeConfig = serde_yaml::from_str(yaml_data).unwrap();

            assert_eq!(config.focus.fg_color, Some(Color::Yellow));
            assert_eq!(config.focus.modifier, Modifier::REVERSED);
            assert_eq!(config.matches.modifier, Modifier::REVERSED);
        }

        /// 下記場合はThemeStyleConfigのdefault値が使われる
        /// ```yaml
        /// focus: {}
        /// matches: {}
        /// ```
        #[test]
        fn deserialize_with_defaults() {
            let yaml_data = indoc! {r#"
                # focus: {}
                # matches: {}
            "#};

            let config: SearchHighlightThemeConfig = serde_yaml::from_str(yaml_data).unwrap();

            assert_eq!(config.focus.fg_color, Some(Color::Yellow));
            assert_eq!(config.focus.modifier, Modifier::REVERSED);
            assert_eq!(config.matches.modifier, Modifier::REVERSED);
        }
    }
}
