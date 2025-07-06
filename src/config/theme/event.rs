use regex::Regex;
use serde::{Deserialize, Serialize};

use super::ThemeStyleConfig;

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EventThemeConfig {
    #[serde(default)]
    pub highlights: Vec<EventHighlightConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventHighlightConfig {
    #[serde(rename = "type", with = "serde_regex")]
    pub ty: Regex,

    pub summary: ThemeStyleConfig,

    pub message: ThemeStyleConfig,
}

mod serde_regex {
    use serde::{Deserialize, Deserializer, Serializer, de};

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

impl PartialEq for EventHighlightConfig {
    fn eq(&self, other: &Self) -> bool {
        self.ty.as_str() == other.ty.as_str()
            && self.summary == other.summary
            && self.message == other.message
    }
}
