use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::LabelColumnConfig;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct NodeThemeConfig {
    pub default_preset: Option<String>,

    /// Each preset is an ordered list of column names: builtin column names
    /// and/or defined label-column names (resolved at load time).
    pub column_presets: Option<HashMap<String, Vec<String>>>,

    /// Registry of label columns: `name` (reference/header) -> `label` (key).
    pub label_columns: Option<Vec<LabelColumnConfig>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserializes_presets_and_label_columns() {
        let json = r#"{
            "default_preset": "gpu",
            "column_presets": { "gpu": ["name", "mig", "status"] },
            "label_columns": [{ "name": "mig", "label": "nvidia.com/mig.config.state" }]
        }"#;
        let cfg: NodeThemeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.default_preset.as_deref(), Some("gpu"));
        assert_eq!(
            cfg.column_presets.as_ref().unwrap().get("gpu").unwrap(),
            &vec!["name".to_string(), "mig".to_string(), "status".to_string()]
        );
        let labels = cfg.label_columns.as_ref().unwrap();
        assert_eq!(labels[0].name, "mig");
        assert_eq!(labels[0].label, "nvidia.com/mig.config.state");
    }
}
