use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::ui::widget::ErrorTheme;

use super::ThemeStyleConfig;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ErrorThemeConfig(pub ThemeStyleConfig);

impl Default for ErrorThemeConfig {
    fn default() -> Self {
        Self(ThemeStyleConfig {
            fg_color: Some(Color::Red),
            ..Default::default()
        })
    }
}

impl From<ErrorThemeConfig> for ErrorTheme {
    fn from(config: ErrorThemeConfig) -> Self {
        ErrorTheme::default().style(config.0)
    }
}
