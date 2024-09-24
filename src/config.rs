pub mod theme;

use std::path::PathBuf;

use anyhow::Result;
use figment::{
    providers::{Format, Serialized, YamlExtended},
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
pub struct Config {
    pub theme: ThemeConfig,
}

impl Config {
    pub fn load(option: ConfigLoadOption) -> Result<Self> {
        let figment = Figment::new();

        let config = match option {
            ConfigLoadOption::Default => figment.merge(Serialized::defaults(Self::default())),
            ConfigLoadOption::Path(path) => figment.merge(YamlExtended::file(path)),
        }
        .extract()?;

        dbg!(&config);

        Ok(config)
    }
}
