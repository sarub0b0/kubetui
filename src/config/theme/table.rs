use serde::{Deserialize, Serialize};

use super::FilterFormThemeConfig;

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TableThemeConfig {
    #[serde(default)]
    pub filter: FilterFormThemeConfig,
}
