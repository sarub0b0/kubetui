mod component;
mod style;
mod tab;

use serde::{Deserialize, Serialize};

pub use self::component::ComponentTheme;
pub use self::tab::TabTheme;
pub use style::{FocusableThemeStyle, ThemeStyle};

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Theme {
    #[serde(default)]
    pub tab: TabTheme,

    #[serde(default)]
    pub component: ComponentTheme,
}

#[cfg(test)]
mod tests {
    use super::*;
}
