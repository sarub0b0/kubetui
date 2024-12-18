mod base;
mod border;
mod dialog;
mod event;
mod filter;
mod header;
mod help;
mod input;
mod list;
mod pod;
mod style;
mod tab;
mod table;
mod text;
mod widget;

use serde::{Deserialize, Serialize};

use crate::features::event::kube::{EventConfig, EventHighlightRule};
use crate::features::pod::kube::{PodConfig, PodHighlightRule};
use crate::ui::dialog::DialogTheme;
use crate::ui::{HeaderTheme, TabTheme};

pub use self::header::HeaderThemeConfig;
pub use self::tab::TabThemeConfig;
pub use base::BaseThemeConfig;
pub use border::BorderThemeConfig;
pub use dialog::*;
pub use event::EventThemeConfig;
pub use filter::*;
pub use help::HelpThemeConfig;
pub use input::*;
pub use list::*;
pub use pod::*;
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
    pub pod: PodThemeConfig,

    #[serde(default)]
    pub event: EventThemeConfig,

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

impl From<ThemeConfig> for PodConfig {
    fn from(theme: ThemeConfig) -> Self {
        PodConfig {
            pod_highlight_rules: theme
                .pod
                .highlights
                .into_iter()
                .map(|hi| PodHighlightRule {
                    status_regex: hi.status,
                    style: hi.style.into(),
                })
                .collect(),
        }
    }
}

impl From<ThemeConfig> for EventConfig {
    fn from(theme: ThemeConfig) -> Self {
        EventConfig {
            highlight_rules: theme
                .event
                .highlights
                .into_iter()
                .map(|hi| EventHighlightRule {
                    ty: hi.ty,
                    summary: hi.summary.into(),
                    message: hi.message.into(),
                })
                .collect(),
        }
    }
}
