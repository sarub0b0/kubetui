mod style;
mod tab;

use serde::{Deserialize, Serialize};

use crate::ui::TabTheme;

pub use self::tab::TabThemeConfig;
pub use style::ThemeStyleConfig;

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ThemeConfig {
    #[serde(default)]
    pub tab: TabThemeConfig,
}

impl From<ThemeConfig> for TabTheme {
    fn from(config: ThemeConfig) -> Self {
        TabTheme::default()
            .divider(config.tab.divider.char)
            .divider_style(config.tab.divider.style)
            .base_style(config.tab.base)
            .active_style(config.tab.active)
            .mouse_over_style(config.tab.mouse_over)
    }
}
