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
