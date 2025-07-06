use std::collections::BTreeMap;

use anyhow::Result;

use crate::kube::{
    KubeClient, KubeClientRequest as _,
    apis::v1_table::{Table, TableRow},
};

#[derive(Debug, Default)]
pub struct KubeTableRow {
    pub namespace: String,
    pub name: String,
    pub metadata: Option<BTreeMap<String, String>>,
    pub row: Vec<String>,
}

#[derive(Debug, Default)]
pub struct KubeTable {
    pub header: Vec<String>,
    pub rows: Vec<KubeTableRow>,
}

#[allow(dead_code)]
impl KubeTable {
    pub fn header(&self) -> &Vec<String> {
        &self.header
    }

    pub fn rows(&self) -> &Vec<KubeTableRow> {
        &self.rows
    }

    pub fn push_row(&mut self, row: impl Into<KubeTableRow>) {
        let row = row.into();

        debug_assert!(
            self.header.len() == row.row.len(),
            "Mismatch header({}) != row({})",
            self.header.len(),
            row.row.len()
        );

        self.rows.push(row);
    }

    pub fn update_rows(&mut self, rows: Vec<KubeTableRow>) {
        if !rows.is_empty() {
            for row in rows.iter() {
                debug_assert!(
                    self.header.len() == row.row.len(),
                    "Mismatch header({}) != row({})",
                    self.header.len(),
                    row.row.len()
                );
            }
        }

        self.rows = rows;
    }
}

#[allow(dead_code)]
pub fn insert_namespace_index(index: usize, len: usize) -> Option<usize> {
    if len != 1 { Some(index) } else { None }
}

pub fn insert_ns(namespaces: &[String]) -> bool {
    namespaces.len() != 1
}

pub async fn get_resource_per_namespace<F>(
    client: &KubeClient,
    path: String,
    target_values: &[&str],
    create_cells: F,
) -> Result<Vec<KubeTableRow>>
where
    F: Fn(&TableRow, &[usize]) -> KubeTableRow,
{
    let table: Table = client.table_request(&path).await?;

    let indexes = table.find_indexes(target_values)?;

    Ok(table
        .rows
        .iter()
        .map(|row| (create_cells)(row, &indexes))
        .collect())
}
