use std::cmp::{Ord, Ordering, PartialOrd};

use anyhow::Result;
use k8s_openapi::apimachinery::pkg::{apis::meta::v1::ListMeta, runtime::RawExtension};
use kube::api::TypeMeta;
use serde::{Deserialize, Deserializer};
use serde_json::Value as JsonValue;

use super::metrics::{NodeMetricsList, PodMetricsList};

#[derive(Default, Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct Value(pub JsonValue);

impl<T> From<T> for Value
where
    T: AsRef<str>,
{
    fn from(s: T) -> Self {
        Value(JsonValue::String(s.as_ref().to_string()))
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    #[serde(flatten)]
    pub type_meta: Option<TypeMeta>,
    pub metadata: Option<ListMeta>,
    pub column_definitions: Vec<TableColumnDefinition>,
    #[serde(deserialize_with = "deserialize_unwrap_or_default")]
    pub rows: Vec<TableRow>,
}

fn deserialize_unwrap_or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableColumnDefinition {
    pub name: String,
    pub r#type: String,
    pub format: String,
    pub description: String,
    pub priority: i32,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub cells: Vec<Value>,
    pub conditions: Option<Vec<TableRowCondition>>,
    pub object: Option<RawExtension>,
}

pub type RowConditionType = String;
pub type ConditionStatus = String;

#[allow(dead_code)]
pub const ROW_COMPLETED: &str = "Completed";

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRowCondition {
    pub r#type: RowConditionType,
    pub status: ConditionStatus,
    pub reason: Option<String>,
    pub message: Option<String>,
}

#[allow(dead_code)]
pub const CONDITION_TRUE: &str = "True";
#[allow(dead_code)]
pub const CONDITION_FALSE: &str = "False";
#[allow(dead_code)]
pub const CONDITION_UNKNOWN: &str = "Unknown";

impl Value {
    #[allow(dead_code)]
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_str()
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            JsonValue::Null => write!(f, "Null"),
            JsonValue::Bool(b) => {
                let s = if *b { "True" } else { "False" };

                write!(f, "{}", s)
            }
            JsonValue::Number(n) => write!(f, "{}", n),
            JsonValue::String(s) => write!(f, "{}", s),
            JsonValue::Array(_) => {
                write!(f, "{}", self.0)
            }
            JsonValue::Object(_) => {
                write!(f, "{}", self.0)
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Time {
    hour: u64,
    minute: u64,
    second: u64,
}
impl Time {
    fn to_second(&self) -> u64 {
        self.second + (self.minute * 60) + (self.hour * 60 * 60)
    }
}

pub trait ToTime {
    fn to_time(&self) -> Time;
}

impl ToTime for str {
    fn to_time(&self) -> Time {
        fn parse(s: &str, unit: char) -> (&str, u64) {
            if let Some(index) = s.find(unit) {
                (&s[(index + 1)..], s[..index].parse().unwrap_or(0))
            } else {
                (s, 0)
            }
        }

        let (s, hour) = parse(self, 'h');
        let (s, minute) = parse(s, 'm');
        let (_s, second) = parse(s, 's');

        Time {
            hour,
            minute,
            second,
        }
    }
}

impl Ord for Time {
    fn cmp(&self, rhs: &Self) -> Ordering {
        rhs.to_second().cmp(&self.to_second())
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Table {
    pub fn find_index(&self, target: &str) -> Option<usize> {
        self.column_definitions
            .iter()
            .position(|cd| cd.name == target)
    }

    pub fn find_indexes(&self, targets: &[&str]) -> Result<Vec<usize>> {
        let mut ret = Vec::with_capacity(targets.len());

        for target in targets {
            if let Some(index) = self.find_index(target) {
                ret.push(index);
            } else {
                let cols = self
                    .column_definitions
                    .iter()
                    .map(|cd| cd.name.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ");

                anyhow::bail!(
                    "Column '{}' not found in table. Available columns: {}",
                    target,
                    cols
                );
            }
        }

        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn sort_rows_by_time(&mut self, time_index: usize) {
        self.rows
            .sort_by_key(|key| key.cells[time_index].to_string().to_time());
    }
}

impl Table {
    pub fn to_print(&self) -> String {
        let header: Vec<(usize, &str)> = self
            .column_definitions
            .iter()
            .enumerate()
            .filter_map(|(i, coldef)| {
                if coldef.priority == 0 {
                    Some((i, coldef.name.as_str()))
                } else {
                    None
                }
            })
            .collect();

        let rows: Vec<Vec<String>> = self
            .rows
            .iter()
            .map(|row| {
                header
                    .iter()
                    .map(|(i, _)| row.cells[*i].to_string())
                    .collect()
            })
            .collect();

        let mut digits: Vec<usize> = header.iter().map(|h| h.1.len()).collect();

        rows.iter().for_each(|cells| {
            cells.iter().enumerate().for_each(|(i, cell)| {
                if digits[i] < cell.len() {
                    digits[i] = cell.len()
                }
            });
        });

        let mut buf = header
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let header_text = h.1.to_uppercase();

                let formatted_header = format!("\x1b[90m{}\x1b[39m", header_text);
                let padding = " ".repeat(digits[i] - header_text.len());

                format!("{}{}", formatted_header, padding)
            })
            .collect::<Vec<String>>()
            .join("   ");

        rows.iter().for_each(|row| {
            buf += &("\n".to_owned()
                + &row
                    .iter()
                    .enumerate()
                    .map(|(i, cell)| format!("{:<digit$}", cell, digit = digits[i]))
                    .collect::<Vec<String>>()
                    .join("   "));
        });

        buf
    }
}

impl From<NodeMetricsList> for Table {
    fn from(list: NodeMetricsList) -> Self {
        Table {
            column_definitions: vec![TableColumnDefinition {
                name: "Name".into(),
                ..Default::default()
            }],
            rows: list
                .names()
                .into_iter()
                .map(|name| TableRow {
                    cells: vec![Value(JsonValue::String(name))],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }
}

impl From<PodMetricsList> for Table {
    fn from(list: PodMetricsList) -> Self {
        Table {
            column_definitions: vec![TableColumnDefinition {
                name: "Name".into(),
                ..Default::default()
            }],
            rows: list
                .names()
                .into_iter()
                .map(|name| TableRow {
                    cells: vec![Value(JsonValue::String(name))],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod find {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn indexes_just() {
            let table = Table {
                type_meta: Default::default(),
                metadata: Default::default(),
                column_definitions: vec![
                    TableColumnDefinition {
                        name: "A".to_string(),
                        ..Default::default()
                    },
                    TableColumnDefinition {
                        name: "B".to_string(),
                        ..Default::default()
                    },
                    TableColumnDefinition {
                        name: "C".to_string(),
                        ..Default::default()
                    },
                    TableColumnDefinition {
                        name: "D".to_string(),
                        ..Default::default()
                    },
                ],
                rows: Default::default(),
            };

            let targets = vec!["A", "B", "C", "D"];

            assert_eq!(table.find_indexes(&targets).unwrap(), vec![0, 1, 2, 3]);
        }

        #[test]
        fn indexes_with_error() {
            let table = Table {
                type_meta: Default::default(),
                metadata: Default::default(),
                column_definitions: vec![
                    TableColumnDefinition {
                        name: "A".to_string(),
                        ..Default::default()
                    },
                    TableColumnDefinition {
                        name: "B".to_string(),
                        ..Default::default()
                    },
                ],
                rows: Default::default(),
            };

            let targets = vec!["A", "B", "C"];

            assert!(table.find_indexes(&targets).is_err());
        }
    }

    mod time {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn to_time_from_3h() {
            assert_eq!(
                "3h".to_time(),
                Time {
                    hour: 3,
                    minute: 0,
                    second: 0
                }
            )
        }

        #[test]
        fn to_time_from_3h10m() {
            assert_eq!(
                "3h10m".to_time(),
                Time {
                    hour: 3,
                    minute: 10,
                    second: 0
                }
            )
        }

        #[test]
        fn to_time_from_10m() {
            assert_eq!(
                "10m".to_time(),
                Time {
                    hour: 0,
                    minute: 10,
                    second: 0
                }
            )
        }

        #[test]
        fn to_second_from_3h10m() {
            assert_eq!("3h10m".to_time().to_second(), 3 * 3600 + 10 * 60)
        }
    }
}
