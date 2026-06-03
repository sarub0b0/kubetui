use serde::{Deserialize, Serialize};

use super::LabelColumnConfig;

/// Theme/config-level settings for the Config tab.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ConfigThemeConfig {
    /// Registry of label columns. All entries are appended to the default
    /// builtin columns at startup (user can toggle them off via the column
    /// dialog).
    pub label_columns: Option<Vec<LabelColumnConfig>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserializes_label_columns() {
        let json = r#"{
            "label_columns": [
                { "name": "app", "label": "app.kubernetes.io/name" },
                { "name": "version", "label": "app.kubernetes.io/version" }
            ]
        }"#;
        let cfg: ConfigThemeConfig = serde_json::from_str(json).unwrap();
        let labels = cfg.label_columns.as_ref().unwrap();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].name, "app");
        assert_eq!(labels[0].label, "app.kubernetes.io/name");
        assert_eq!(labels[1].name, "version");
        assert_eq!(labels[1].label, "app.kubernetes.io/version");
    }

    #[test]
    fn default_has_none_label_columns() {
        let cfg = ConfigThemeConfig::default();
        assert!(cfg.label_columns.is_none());
    }
}
