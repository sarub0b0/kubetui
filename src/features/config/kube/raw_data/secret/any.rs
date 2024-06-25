use std::collections::{btree_map, BTreeMap};

use base64::{engine::general_purpose, Engine};
use k8s_openapi::ByteString;

use crate::workers::kube::color::Color;

use super::format::{format_error, format_utf8};

/// any type secret
#[derive(Debug, Default)]
pub struct Any {
    data: BTreeMap<String, ByteString>,
}

impl Any {
    pub fn new(data: BTreeMap<String, ByteString>) -> Self {
        Self { data }
    }

    pub fn to_string_key_values(&self) -> Vec<String> {
        self.iter()
            .flat_map(|key_value| {
                key_value
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
            })
            .collect()
    }

    fn iter(&self) -> Iter {
        Iter {
            iter: self.data.iter(),
            color: Color::new(),
        }
    }
}

struct Iter<'a> {
    iter: btree_map::Iter<'a, String, ByteString>,
    color: Color,
}

impl Iterator for Iter<'_> {
    type Item = String;
    fn next(&mut self) -> std::option::Option<<Self as Iterator>::Item> {
        let (key, ByteString(value)) = self.iter.next()?;

        let color = self.color.next_color();

        match String::from_utf8(value.to_vec()) {
            Ok(utf8_data) => Some(format_utf8(key, &utf8_data, color)),
            Err(err) => {
                let base64_encoded = general_purpose::STANDARD.encode(value);

                Some(format_error(key, &base64_encoded, &err.to_string(), color))
            }
        }
    }
}
