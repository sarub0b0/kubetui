use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

/// Configuration structure for customizing the appearance and behavior of the `CheckList` widget.
///
/// This struct is typically deserialized from a YAML configuration file.
/// Each field corresponds to a visual or symbolic aspect of the checklist UI.
///
/// # Example (YAML)
/// ```yaml
/// check_list:
///   selected:
///     fg_color: green
///   selected_symbol: "→"
///   required:
///     fg_color: red
///   required_symbol: "✗"
///   checked_symbol: "✓"
///   unchecked_symbol: "☐"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CheckListThemeConfig {
    /// Style applied to the currently selected (focused) item.
    #[serde(default = "default_selected")]
    pub selected: ThemeStyleConfig,

    /// Symbol shown before the selected item (e.g., "→", ">", "*").
    #[serde(default = "default_selected_symbol")]
    pub selected_symbol: String,

    /// Style applied to required (non-deselectable) items.
    #[serde(default = "default_required")]
    pub required: ThemeStyleConfig,

    /// Symbol shown next to required items (e.g., "✗", "required", "[!]").
    #[serde(default = "default_required_symbol")]
    pub required_symbol: String,

    /// Symbol shown when an item is checked (e.g., "✓", "[x]").
    #[serde(default = "default_checked_symbol")]
    pub checked_symbol: String,

    /// Symbol shown when an item is not checked (e.g., "☐", "[ ]").
    #[serde(default = "default_unchecked_symbol")]
    pub unchecked_symbol: String,
}

impl Default for CheckListThemeConfig {
    fn default() -> Self {
        Self {
            selected: default_selected(),
            selected_symbol: default_selected_symbol(),
            required: default_required(),
            required_symbol: default_required_symbol(),
            checked_symbol: default_checked_symbol(),
            unchecked_symbol: default_unchecked_symbol(),
        }
    }
}

fn default_selected() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::REVERSED,
        ..Default::default()
    }
}

fn default_required() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::DarkGray),
        ..Default::default()
    }
}

fn default_required_symbol() -> String {
    "(required)".to_string()
}

fn default_selected_symbol() -> String {
    ">".to_string()
}

fn default_checked_symbol() -> String {
    "[x]".to_string()
}

fn default_unchecked_symbol() -> String {
    "[ ]".to_string()
}
