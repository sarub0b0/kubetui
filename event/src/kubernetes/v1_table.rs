use k8s_openapi::apimachinery::pkg::apis::meta::v1::ListMeta;
use k8s_openapi::apimachinery::pkg::runtime::RawExtension;
use kube::{api::TypeMeta, Client, Result};
use serde::Deserialize;

use serde_json::Value as JsonValue;
use std::cmp::Ordering;
use std::cmp::{Ord, PartialOrd};

use super::request::get_table_request;

#[derive(Default, Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct Value(pub JsonValue);

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub list_meta: Option<ListMeta>,
    pub column_definitions: Vec<TableColumnDefinition>,
    pub rows: Vec<TableRow>,
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

impl ToString for Value {
    fn to_string(&self) -> String {
        match &self.0 {
            JsonValue::Null => "Null".to_string(),
            JsonValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::String(s) => s.to_string(),
            JsonValue::Array(_) => {
                format!("{}", self.0)
            }
            JsonValue::Object(_) => {
                format!("{}", self.0)
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

        let (s, hour) = parse(&self, 'h');
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

    pub fn find_indexes(&self, targets: &[&str]) -> Vec<usize> {
        targets
            .iter()
            .filter_map(|target| self.find_index(target))
            .collect()
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
                format!(
                    "\x1b[90m{:<digit$}\x1b[0m",
                    h.1.to_uppercase(),
                    digit = digits[i]
                )
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

#[allow(dead_code)]
pub fn insert_namespace_index(index: usize, len: usize) -> Option<usize> {
    if len != 1 {
        Some(index)
    } else {
        None
    }
}

pub fn insert_ns(namespaces: &[String]) -> bool {
    namespaces.len() != 1
}

pub async fn get_resourse_per_namespace<F>(
    client: &Client,
    server_url: &str,
    path: String,
    target_values: &[&str],
    create_cells: F,
) -> Result<Vec<Vec<String>>>
where
    F: Fn(&TableRow, &[usize]) -> Vec<String>,
{
    let table: Result<Table, kube::Error> = client
        .request(get_table_request(server_url, &path).unwrap())
        .await;

    match table {
        Ok(t) => {
            let indexes = t.find_indexes(target_values);

            Ok(t.rows
                .iter()
                .map(|row| (create_cells)(&row, &indexes))
                .collect())
        }
        Err(e) => Err(e),
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
                list_meta: None,
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

            assert_eq!(table.find_indexes(&targets), vec![0, 1, 2, 3]);

            let targets = vec!["A", "B", "E"];

            assert_eq!(table.find_indexes(&targets), vec![0, 1]);
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
