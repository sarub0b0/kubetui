pub mod theme;

use std::path::PathBuf;

use anyhow::Result;
use figment::{
    providers::{Env, Format, Serialized, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};

use theme::ThemeConfig;

#[derive(Debug, Default)]
pub enum ConfigLoadOption {
    #[default]
    Default,

    Path(PathBuf),
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub max_lines: Option<usize>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub fallback_namespaces: Option<Vec<String>>,
}

impl Config {
    pub fn load(option: ConfigLoadOption) -> Result<Self> {
        let figment = Figment::new();

        let config = match option {
            ConfigLoadOption::Default => figment.merge(Serialized::defaults(Self::default())),
            ConfigLoadOption::Path(path) => figment
                .merge(Serialized::defaults(Self::default()))
                .merge(Yaml::file(path)),
        }
        .merge(Env::prefixed("KUBETUI_").split("__"))
        .extract_lossy()?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn fallback_namespaces_が設定されている場合() {
        let yaml = indoc! {"
            fallback_namespaces:
              - production
              - staging
              - dev
        "};
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.fallback_namespaces,
            Some(vec![
                "production".to_string(),
                "staging".to_string(),
                "dev".to_string(),
            ])
        );
    }

    #[test]
    fn fallback_namespaces_が未設定の場合() {
        let yaml = indoc! {"
            logging:
              max_lines: 1000
        "};
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.fallback_namespaces, None);
    }

    #[test]
    fn fallback_namespaces_が空配列の場合() {
        let yaml = indoc! {"
            fallback_namespaces: []
        "};
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.fallback_namespaces, Some(vec![]));
    }
}
