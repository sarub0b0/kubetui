use serde::{Deserialize, Serialize};

/// User-declared label-column entry shared across tabs (Pod, Node, etc.).
///
/// `name` becomes the column header (uppercased) and the filter identifier.
/// `label` is the Kubernetes label key whose value is rendered in the cell.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LabelColumnConfig {
    pub name: String,
    pub label: String,
}
