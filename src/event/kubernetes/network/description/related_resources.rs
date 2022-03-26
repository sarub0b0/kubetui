#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Result;
use serde_yaml::Value;

pub mod pod;

trait RelatedResources {
    fn related_resources(&self) -> Result<Option<Value>>;
}
