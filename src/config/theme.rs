mod base;
mod border;
mod header;
mod style;
mod tab;
mod text;
mod widget;

use serde::{Deserialize, Serialize};

use crate::ui::{HeaderTheme, TabTheme};

pub use self::header::HeaderThemeConfig;
pub use self::tab::TabThemeConfig;
pub use base::BaseThemeConfig;
pub use border::BorderThemeConfig;
pub use style::ThemeStyleConfig;
pub use text::*;
pub use widget::WidgetThemeConfig;

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ThemeConfig {
    #[serde(default)]
    pub base: BaseThemeConfig,

    #[serde(default)]
    pub tab: TabThemeConfig,

    #[serde(default)]
    pub header: HeaderThemeConfig,

    #[serde(default)]
    pub component: WidgetThemeConfig,
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

impl From<ThemeConfig> for HeaderTheme {
    fn from(config: ThemeConfig) -> Self {
        HeaderTheme::default()
            .base_style(config.header.base)
            .line_styles([config.header.cluster, config.header.namespaces])
    }
}
