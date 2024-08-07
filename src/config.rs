use std::path::PathBuf;

use anyhow::Result;
use figment::{
    providers::{Format, Serialized, YamlExtended},
    Figment,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub enum ConfigLoadOption {
    #[default]
    Default,

    Path(PathBuf),
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Config {}

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
