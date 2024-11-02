use ratatui::style::Modifier;
use serde::{Deserialize, Serialize};

use super::{FilterFormThemeConfig, ThemeStyleConfig};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ListThemeConfig {
    #[serde(default)]
    pub filter: FilterFormThemeConfig,

    #[serde(default = "default_selection")]
    pub selection: ThemeStyleConfig,

    #[serde(default)]
    pub status: ThemeStyleConfig,
}

impl Default for ListThemeConfig {
    fn default() -> ListThemeConfig {
        ListThemeConfig {
            selection: default_selection(),
            filter: FilterFormThemeConfig::default(),
            status: Default::default(),
        }
    }
}

fn default_selection() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::REVERSED,
        ..Default::default()
    }
}
