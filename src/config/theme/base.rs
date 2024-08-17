use std::ops::Deref;

use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BaseThemeConfig(#[serde(default)] pub ThemeStyleConfig);

impl Deref for BaseThemeConfig {
    type Target = ThemeStyleConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BaseThemeConfig> for ratatui::style::Style {
    fn from(config: BaseThemeConfig) -> Self {
        config.0.into()
    }
}
