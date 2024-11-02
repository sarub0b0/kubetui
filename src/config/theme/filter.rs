use serde::{Deserialize, Serialize};

use super::{BorderThemeConfig, ThemeStyleConfig};

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FilterFormThemeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<ThemeStyleConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<BorderThemeConfig>,

    #[serde(default)]
    pub prefix: ThemeStyleConfig,

    #[serde(default)]
    pub query: ThemeStyleConfig,
}
