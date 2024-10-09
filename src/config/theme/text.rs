mod search;
mod selection;

pub use search::*;
pub use selection::*;

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TextThemeConfig {
    #[serde(default)]
    pub search: SearchThemeConfig,

    #[serde(default)]
    pub selection: SelectionThemeConfig,
}
