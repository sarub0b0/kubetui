use ratatui::style::Modifier;
use serde::{Deserialize, Serialize};

use super::{FilterFormThemeConfig, ThemeStyleConfig};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ListThemeConfig {
    #[serde(default)]
    pub filter: FilterFormThemeConfig,

    #[serde(default = "default_selected_item")]
    pub selected_item: ThemeStyleConfig,

    #[serde(default)]
    pub status: ThemeStyleConfig,
}

impl Default for ListThemeConfig {
    fn default() -> ListThemeConfig {
        ListThemeConfig {
            selected_item: default_selected_item(),
            filter: FilterFormThemeConfig::default(),
            status: Default::default(),
        }
    }
}

fn default_selected_item() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::REVERSED,
        ..Default::default()
    }
}
