use serde::{Deserialize, Serialize};

use crate::ui::dialog::DialogSize;

use super::ThemeStyleConfig;

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct DialogThemeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<ThemeStyleConfig>,

    #[serde(default)]
    pub size: DialogSizeThemeConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct DialogSizeThemeConfig {
    #[serde(default = "default_width")]
    pub width: f32,

    #[serde(default = "default_height")]
    pub height: f32,
}

impl Default for DialogSizeThemeConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
        }
    }
}

fn default_width() -> f32 {
    85.0
}

fn default_height() -> f32 {
    85.0
}

impl From<DialogSizeThemeConfig> for DialogSize {
    fn from(config: DialogSizeThemeConfig) -> Self {
        Self {
            width: config.width,
            height: config.height,
        }
    }
}
