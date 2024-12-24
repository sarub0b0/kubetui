use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiThemeConfig {
    #[serde(default)]
    pub table: ApiTableThemeConfig,
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
