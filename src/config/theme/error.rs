use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::ui::widget::ErrorTheme;

use super::ThemeStyleConfig;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ErrorThemeConfig {
    #[serde(default = "default_error_style")]
    pub style: ThemeStyleConfig,
}

impl Default for ErrorThemeConfig {
    fn default() -> Self {
        Self {
            style: default_error_style(),
        }
    }
}

fn default_error_style() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::Red),
        ..Default::default()
    }
}

impl From<ErrorThemeConfig> for ErrorTheme {
    fn from(config: ErrorThemeConfig) -> Self {
        ErrorTheme::default().style(config.style)
    }
}
