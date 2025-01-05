use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct YamlThemeConfig {
    #[serde(default)]
    pub dialog: YamlDialogThemeConfig,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct YamlDialogThemeConfig {
    #[serde(default)]
    pub preferred_version_or_latest: ThemeStyleConfig,

    #[serde(default = "default_other_version_style")]
    pub other_version: ThemeStyleConfig,
}

impl Default for YamlDialogThemeConfig {
    fn default() -> Self {
        Self {
            preferred_version_or_latest: ThemeStyleConfig::default(),
            other_version: default_other_version_style(),
        }
    }
}

fn default_other_version_style() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::DarkGray),
        ..Default::default()
    }
}
