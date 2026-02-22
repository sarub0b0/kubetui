pub mod theme;

use std::path::PathBuf;

use anyhow::Result;
use figment::{
    providers::{Env, Format, Serialized, YamlExtended},
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
    pub theme: ThemeConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn load(option: ConfigLoadOption) -> Result<Self> {
        let figment = Figment::new();

        let config = match option {
            ConfigLoadOption::Default => figment.merge(Serialized::defaults(Self::default())),
            ConfigLoadOption::Path(path) => figment.merge(YamlExtended::file(path)),
        }
        .merge(Env::prefixed("KUBETUI_").split("__"))
        .extract_lossy()?;

        Ok(config)
    }
}
