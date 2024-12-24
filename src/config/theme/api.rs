use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiThemeConfig {
    #[serde(default)]
    pub table: ApiTableThemeConfig,

    #[serde(default)]
    pub dialog: ApiDialogThemeConfig,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiTableThemeConfig {
    #[serde(default = "default_resource_style")]
    pub resource: ThemeStyleConfig,

    #[serde(default = "default_header_style")]
    pub header: ThemeStyleConfig,

    #[serde(default)]
    pub rows: ThemeStyleConfig,
}

impl Default for ApiTableThemeConfig {
    fn default() -> Self {
        Self {
            resource: default_resource_style(),
            header: default_header_style(),
            rows: ThemeStyleConfig::default(),
        }
    }
}

fn default_resource_style() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::DarkGray),
        ..Default::default()
    }
}

fn default_header_style() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::DarkGray),
        ..Default::default()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiDialogThemeConfig {
    #[serde(default)]
    pub preferred_version_or_latest: ThemeStyleConfig,

    #[serde(default = "default_other_version_style")]
    pub other_version: ThemeStyleConfig,
}

impl Default for ApiDialogThemeConfig {
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
