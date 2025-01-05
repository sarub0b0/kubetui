use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

/// ヘッダーのテーマ
#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct HeaderThemeConfig {
    #[serde(default)]
    pub base: ThemeStyleConfig,

    #[serde(default)]
    pub cluster: ThemeStyleConfig,

    #[serde(default)]
    pub namespaces: ThemeStyleConfig,
}
