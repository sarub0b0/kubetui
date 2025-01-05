use ratatui::{
    style::{Color, Modifier},
    symbols,
};
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

/// タブのテーマ
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TabThemeConfig {
    #[serde(default)]
    pub divider: TabDividerConfig,

    #[serde(default)]
    pub base: ThemeStyleConfig,

    #[serde(default = "default_active")]
    pub active: ThemeStyleConfig,

    #[serde(default = "default_mouse_over")]
    pub mouse_over: ThemeStyleConfig,
}

impl Default for TabThemeConfig {
    fn default() -> Self {
        Self {
            divider: TabDividerConfig::default(),
            base: ThemeStyleConfig::default(),
            active: default_active(),
            mouse_over: default_mouse_over(),
        }
    }
}

fn default_active() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::REVERSED,
        ..Default::default()
    }
}

fn default_mouse_over() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::REVERSED,
        fg_color: Some(Color::DarkGray),
        ..Default::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TabDividerConfig {
    pub char: String,

    #[serde(flatten)]
    pub style: ThemeStyleConfig,
}

impl Default for TabDividerConfig {
    fn default() -> Self {
        Self {
            char: symbols::line::VERTICAL.to_string(),
            style: ThemeStyleConfig::default(),
        }
    }
}
