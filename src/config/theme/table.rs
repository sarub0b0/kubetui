use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::{FilterFormThemeConfig, ThemeStyleConfig};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TableThemeConfig {
    #[serde(default)]
    pub filter: FilterFormThemeConfig,

    #[serde(default = "default_table_header")]
    pub header: ThemeStyleConfig,
}

impl Default for TableThemeConfig {
    fn default() -> Self {
        Self {
            filter: Default::default(),
            header: default_table_header(),
        }
    }
}

fn default_table_header() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::DarkGray),
        ..Default::default()
    }
}
