use std::ops::Deref;

use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct InputFormThemeConfig(pub ThemeStyleConfig);

impl Deref for InputFormThemeConfig {
    type Target = ThemeStyleConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<InputFormThemeConfig> for ratatui::style::Style {
    fn from(config: InputFormThemeConfig) -> Self {
        config.0.into()
    }
}
