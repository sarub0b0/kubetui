use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::features::node::NodeColumn;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct NodeThemeConfig {
    pub default_preset: Option<String>,

    pub column_presets: Option<HashMap<String, Vec<NodeColumnConfig>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NodeColumnConfig(#[serde(with = "serde_node_column")] pub NodeColumn);

mod serde_node_column {
    use std::str::FromStr as _;

    use serde::{de, Deserialize, Deserializer, Serializer};

    use crate::features::node::NodeColumn;

    pub fn serialize<S>(column: &NodeColumn, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&NodeColumn::normalize_column(column.as_str()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NodeColumn, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NodeColumn::from_str(&s).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::node::NodeColumn;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserializes_column_presets() {
        let json = r#"{
            "default_preset": "default",
            "column_presets": { "default": ["name", "status", "roles", "age", "version"] }
        }"#;
        let cfg: NodeThemeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.default_preset.as_deref(), Some("default"));
        let preset = cfg.column_presets.as_ref().unwrap().get("default").unwrap();
        let cols: Vec<NodeColumn> = preset.iter().map(|c| c.0).collect();
        assert_eq!(
            cols,
            vec![
                NodeColumn::Name,
                NodeColumn::Status,
                NodeColumn::Roles,
                NodeColumn::Age,
                NodeColumn::Version
            ]
        );
    }
}
