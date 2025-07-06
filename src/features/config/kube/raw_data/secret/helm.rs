use std::{collections::BTreeMap, io::prelude::*};

use anyhow::Result;
use base64::{Engine, engine::general_purpose};
use k8s_openapi::ByteString;

use crate::workers::kube::color::{self, Color};

use super::format::{format_error, format_utf8};

#[derive(Debug, Default)]
pub struct Helm {
    data: BTreeMap<String, ByteString>,
}

impl Helm {
    pub fn new(data: BTreeMap<String, ByteString>) -> Self {
        Self { data }
    }

    pub fn to_string_key_values(&self) -> Vec<String> {
        let Some(ByteString(value)) = self.data.get("release") else {
            return vec!["no release data".into()];
        };

        let mut color = Color::new();

        let decoded_release = match decode_release(value) {
            Ok(decoded) => {
                let color = color.next_color();
                format_utf8("release (decoded)", &decoded, color)
            }
            Err(err) => {
                format!(
                    "\x1b[{red}m# Failed to decode the 'release' value: {err}\x1b[39m",
                    red = color::fg::Color::Red as u8,
                    err = err
                )
            }
        };

        let color = color.next_color();

        let release = match String::from_utf8(value.to_vec()) {
            Ok(utf8_data) => format_utf8("release", &utf8_data, color),
            Err(err) => {
                let base64_encoded = general_purpose::STANDARD.encode(value);
                format_error("release", &base64_encoded, &err.to_string(), color)
            }
        };

        decoded_release
            .lines()
            .chain(release.lines())
            .map(ToString::to_string)
            .collect()
    }
}

fn decode_release(data: &[u8]) -> Result<String> {
    let gzip = general_purpose::STANDARD.decode(data)?;

    // decode gzip
    let mut decoder = flate2::read::GzDecoder::new(&gzip[..]);
    let mut decoded = String::new();
    decoder.read_to_string(&mut decoded)?;

    let yaml = serde_yaml::from_str::<serde_yaml::Value>(&decoded)?;

    serde_yaml::to_string(&yaml).map_err(Into::into)
}
