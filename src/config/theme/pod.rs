use ratatui::style::Color;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PodThemeConfig {
    #[serde(default = "default_pod_statuses")]
    pub status_patterns: Vec<PodStatusPatternConfig>,
}

impl Default for PodThemeConfig {
    fn default() -> Self {
        Self {
            status_patterns: default_pod_statuses(),
        }
    }
}

// defualt

fn default_pod_statuses() -> Vec<PodStatusPatternConfig> {
    vec![
        PodStatusPatternConfig {
            pattern: Regex::new(r"(Completed|Evicted)").unwrap(),
            style: ThemeStyleConfig {
                fg_color: Some(Color::DarkGray),
                ..Default::default()
            },
        },
        PodStatusPatternConfig {
            pattern: Regex::new(r"(BackOff|Err|Unknown)").unwrap(),
            style: ThemeStyleConfig {
                fg_color: Some(Color::Red),
                ..Default::default()
            },
        },
    ]
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PodStatusPatternConfig {
    #[serde(with = "serde_regex")]
    pub pattern: Regex,

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

impl PartialEq for PodStatusPatternConfig {
    fn eq(&self, other: &Self) -> bool {
        self.pattern.as_str() == other.pattern.as_str() && self.style == other.style
    }
}
