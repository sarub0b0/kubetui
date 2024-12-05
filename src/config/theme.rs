mod base;
mod border;
mod dialog;
mod filter;
mod header;
mod help;
mod input;
mod list;
mod style;
mod tab;
mod table;
mod text;
mod widget;

use serde::{Deserialize, Serialize};

use crate::ui::dialog::DialogTheme;
use crate::ui::{HeaderTheme, TabTheme};

pub use self::header::HeaderThemeConfig;
pub use self::tab::TabThemeConfig;
pub use base::BaseThemeConfig;
pub use border::BorderThemeConfig;
pub use dialog::*;
pub use filter::*;
pub use help::HelpThemeConfig;
pub use input::*;
pub use list::*;
pub use style::ThemeStyleConfig;
pub use table::*;
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

    #[serde(default)]
    pub help: HelpThemeConfig,
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

impl From<ThemeConfig> for DialogTheme {
    fn from(config: ThemeConfig) -> Self {
        let base_style = config.component.dialog.base.unwrap_or_else(|| *config.base);

        DialogTheme::default()
            .base_style(base_style)
            .size(config.component.dialog.size)
    }
}
