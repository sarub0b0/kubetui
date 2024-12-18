use ratatui::style::Color;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PodThemeConfig {
    #[serde(default = "default_highlights")]
    pub highlights: Vec<PodHighlightConfig>,
}

impl Default for PodThemeConfig {
    fn default() -> Self {
        Self {
            highlights: default_highlights(),
        }
    }
}

fn default_highlights() -> Vec<PodHighlightConfig> {
    vec![
        PodHighlightConfig {
            status: Regex::new(r"(Completed|Evicted)").unwrap(),
            style: ThemeStyleConfig {
                fg_color: Some(Color::DarkGray),
                ..Default::default()
            },
        },
        PodHighlightConfig {
            status: Regex::new(r"(BackOff|Err|Unknown)").unwrap(),
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
