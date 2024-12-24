use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};

use crate::features::help::HelpItemTheme;

use super::ThemeStyleConfig;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HelpThemeConfig {
    #[serde(default = "default_title_style")]
    pub title: ThemeStyleConfig,

    #[serde(default = "default_key_style")]
    pub key: ThemeStyleConfig,

    #[serde(default, alias = "description", alias = "desc")]
    pub desc: ThemeStyleConfig,
}

fn default_title_style() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::BOLD,
        ..Default::default()
    }
}

fn default_key_style() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::LightCyan),
        ..Default::default()
    }
}

impl Default for HelpThemeConfig {
    fn default() -> Self {
        Self {
            title: default_title_style(),
            key: default_key_style(),
            desc: Default::default(),
        }
    }
}

impl From<HelpThemeConfig> for HelpItemTheme {
    fn from(config: HelpThemeConfig) -> Self {
        HelpItemTheme {
            title_style: config.title.into(),
            key_style: config.key.into(),
            desc_style: config.desc.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use ratatui::style::{Color, Modifier};
    use serde_yaml;

    #[test]
    fn test_help_theme_config_defaults() {
        let config = HelpThemeConfig::default();

        // Check default title style
        assert_eq!(config.title.modifier, Modifier::BOLD);
        assert_eq!(config.title.fg_color, None);
        assert_eq!(config.title.bg_color, None);

        // Check default key style
        assert_eq!(config.key.fg_color, Some(Color::LightCyan));
        assert_eq!(config.key.modifier, Modifier::empty());
        assert_eq!(config.key.bg_color, None);

        // Check default desc style
        assert_eq!(config.desc.fg_color, None);
        assert_eq!(config.desc.modifier, Modifier::empty());
        assert_eq!(config.desc.bg_color, None);
    }

    #[test]
    fn test_help_theme_config_yaml_serialization() {
        let config = HelpThemeConfig {
            title: ThemeStyleConfig {
                fg_color: Some(Color::Yellow),
                ..Default::default()
            },
            key: ThemeStyleConfig {
                fg_color: Some(Color::Cyan),
                ..Default::default()
            },
            desc: ThemeStyleConfig {
                fg_color: Some(Color::Gray),
                ..Default::default()
            },
        };
        let serialized = serde_yaml::to_string(&config).unwrap();

        // Expected YAML string
        let expected_yaml = indoc! { r#"
            title:
              fg_color: yellow
            key:
              fg_color: cyan
            desc:
              fg_color: gray
        "#};

        assert_eq!(serialized, expected_yaml);
    }

    #[test]
    fn test_help_theme_config_yaml_deserialization() {
        let yaml_str = indoc! { r#"
            title:
              fg_color: yellow
            key:
              fg_color: cyan
            desc:
              fg_color: gray
        "# };

        let deserialized: HelpThemeConfig = serde_yaml::from_str(yaml_str).unwrap();
        let expected = HelpThemeConfig {
            title: ThemeStyleConfig {
                fg_color: Some(Color::Yellow),
                ..Default::default()
            },
            key: ThemeStyleConfig {
                fg_color: Some(Color::Cyan),
                ..Default::default()
            },
            desc: ThemeStyleConfig {
                fg_color: Some(Color::Gray),
                ..Default::default()
            },
        };

        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_help_theme_config_from() {
        let config = HelpThemeConfig::default();
        let help_item_theme: HelpItemTheme = config.clone().into();

        assert_eq!(help_item_theme.title_style, config.title.into());
        assert_eq!(help_item_theme.key_style, config.key.into());
        assert_eq!(help_item_theme.desc_style, config.desc.into());
    }

    #[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
    struct Nested {
        #[serde(default)]
        help: HelpThemeConfig,
    }
}
