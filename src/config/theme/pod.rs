use std::collections::HashMap;

use ratatui::style::Color;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{node::LabelColumnConfig, ThemeStyleConfig};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PodThemeConfig {
    #[serde(default = "default_highlights")]
    pub highlights: Vec<PodHighlightConfig>,

    pub default_preset: Option<String>,

    pub column_presets: Option<HashMap<String, Vec<String>>>,

    pub label_columns: Option<Vec<LabelColumnConfig>>,
}

impl Default for PodThemeConfig {
    fn default() -> Self {
        Self {
            highlights: default_highlights(),
            default_preset: None,
            column_presets: None,
            label_columns: None,
        }
    }
}

fn default_highlights() -> Vec<PodHighlightConfig> {
    vec![
        PodHighlightConfig {
            status: Regex::new(r"(Completed|Evicted)").expect("invalid regex"),
            style: ThemeStyleConfig {
                fg_color: Some(Color::DarkGray),
                ..Default::default()
            },
        },
        PodHighlightConfig {
            status: Regex::new(r"(BackOff|Err|Unknown)").expect("invalid regex"),
            style: ThemeStyleConfig {
                fg_color: Some(Color::Red),
                ..Default::default()
            },
        },
    ]
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PodHighlightConfig {
    #[serde(with = "serde_regex")]
    pub status: Regex,

    #[serde(flatten)]
    pub style: ThemeStyleConfig,
}

mod serde_regex {
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(regex: &regex::Regex, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&regex.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<regex::Regex, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        regex::Regex::new(&s).map_err(de::Error::custom)
    }
}

impl PartialEq for PodHighlightConfig {
    fn eq(&self, other: &Self) -> bool {
        self.status.as_str() == other.status.as_str() && self.style == other.style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserializes_label_columns_and_string_presets() {
        let json = r#"{
            "column_presets": { "wide": ["name", "status", "version"] },
            "label_columns": [{ "name": "version", "label": "app.kubernetes.io/version" }]
        }"#;
        let cfg: PodThemeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            cfg.column_presets.as_ref().unwrap().get("wide").unwrap(),
            &vec!["name".to_string(), "status".to_string(), "version".to_string()]
        );
        let labels = cfg.label_columns.as_ref().unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "version");
        assert_eq!(labels[0].label, "app.kubernetes.io/version");
    }
}
