use std::ops::Deref;

use ratatui::style::Modifier;
use serde::{Deserialize, Serialize};

use crate::{config::theme::ThemeStyleConfig, ui::widget::SelectionStyle};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SelectionThemeConfig(#[serde(default = "default_style")] pub ThemeStyleConfig);

impl Default for SelectionThemeConfig {
    fn default() -> Self {
        Self(default_style())
    }
}

fn default_style() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::REVERSED,
        ..Default::default()
    }
}

impl Deref for SelectionThemeConfig {
    type Target = ThemeStyleConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SelectionThemeConfig> for ratatui::style::Style {
    fn from(config: SelectionThemeConfig) -> Self {
        config.0.into()
    }
}

impl From<SelectionThemeConfig> for SelectionStyle {
    fn from(config: SelectionThemeConfig) -> Self {
        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml;

    #[test]
    fn test_default_selection_theme_config() {
        let default_config = SelectionThemeConfig::default();

        assert_eq!(default_config.modifier, Modifier::REVERSED);
    }

    #[test]
    fn test_deserialize_selection_theme_config() {
        let yaml_data = r#"
        modifier: REVERSED
        "#;

        let config: SelectionThemeConfig = serde_yaml::from_str(yaml_data).unwrap();

        assert_eq!(config.modifier, Modifier::REVERSED);
    }
}
