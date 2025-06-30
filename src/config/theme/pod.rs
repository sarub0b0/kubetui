use std::collections::HashMap;

use ratatui::style::Color;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::features::pod::{PodColumn, PodColumns};

use super::ThemeStyleConfig;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PodThemeConfig {
    #[serde(default = "default_highlights")]
    pub highlights: Vec<PodHighlightConfig>,

    pub default_preset: Option<String>,

    pub column_presets: Option<HashMap<String, Vec<PodColumnConfig>>>,
}

impl Default for PodThemeConfig {
    fn default() -> Self {
        Self {
            highlights: default_highlights(),
            default_preset: None,
            column_presets: None,
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PodColumnConfig(#[serde(with = "serde_pod_column")] pub PodColumn);

mod serde_pod_column {
    use std::str::FromStr as _;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(column: &super::PodColumn, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(column.normalize())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<super::PodColumn, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        super::PodColumn::from_str(&s).map_err(de::Error::custom)
    }
}

impl<T: AsRef<[PodColumnConfig]>> From<T> for PodColumns {
    fn from(value: T) -> Self {
        PodColumns::new(value.as_ref().iter().map(|c| c.0))
    }
}
