use k8s_openapi::apimachinery::pkg::apis::meta::v1::ListMeta;
use k8s_openapi::apimachinery::pkg::runtime::RawExtension;
use kube::api::TypeMeta;
use serde::Deserialize;

use serde_json::Value;

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub list_meta: Option<ListMeta>,
    pub column_definitions: Vec<TableColumnDefinition>,
    pub rows: Vec<TableRow>,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableColumnDefinition {
    pub name: String,
    pub r#type: String,
    pub format: String,
    pub description: String,
    pub priority: i32,
}

#[derive(Default, Debug, Deserialize)]
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

#[derive(Default, Debug, Deserialize)]
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
}

fn to_print_value(value: &Value) -> String {
    match value {
        Value::Null => "Null".to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.to_string(),
        Value::Array(_) => {
            format!("{}", value)
        }
        Value::Object(_) => {
            format!("{}", value)
        }
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
                    .map(|(i, _)| to_print_value(&row.cells[*i]))
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
}
