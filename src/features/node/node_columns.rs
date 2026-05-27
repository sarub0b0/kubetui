use strum::{EnumIter, IntoEnumIterator};

/// A runtime column in the node table: a built-in column or a label column.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeColumnSpec {
    Builtin(NodeColumn),
    Label { key: String, header: String },
}

impl NodeColumnSpec {
    /// Display header (uppercase). Builtin uses display(), Label uses its header.
    pub fn header(&self) -> String {
        match self {
            NodeColumnSpec::Builtin(c) => c.display().to_string(),
            NodeColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

/// A resolved label-column definition (an entry of the label registry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeColumns {
    columns: Vec<NodeColumnSpec>,
}

impl Default for NodeColumns {
    fn default() -> Self {
        NodeColumns {
            columns: DEFAULT_NODE_COLUMNS
                .iter()
                .map(|c| NodeColumnSpec::Builtin(*c))
                .collect(),
        }
    }
}

#[allow(dead_code)]
impl NodeColumns {
    pub fn new(columns: impl IntoIterator<Item = NodeColumnSpec>) -> Self {
        NodeColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn from_builtins(columns: impl IntoIterator<Item = NodeColumn>) -> Self {
        NodeColumns {
            columns: columns.into_iter().map(NodeColumnSpec::Builtin).collect(),
        }
    }

    pub fn specs(&self) -> &[NodeColumnSpec] {
        &self.columns
    }

    pub fn ensure_name_column(mut self) -> Self {
        let has_name = self
            .columns
            .iter()
            .any(|s| matches!(s, NodeColumnSpec::Builtin(NodeColumn::Name)));
        if !has_name {
            self.columns
                .insert(0, NodeColumnSpec::Builtin(NodeColumn::Name));
        }
        self
    }

    // Removes duplicates while preserving order.
    // Linear search is used because the number of columns is small.
    pub fn dedup_columns(self) -> Self {
        let mut unique: Vec<NodeColumnSpec> = Vec::new();
        for c in self.columns {
            if !unique.contains(&c) {
                unique.push(c);
            }
        }
        NodeColumns { columns: unique }
    }
}

pub const DEFAULT_NODE_COLUMNS: &[NodeColumn] = &[
    NodeColumn::Name,
    NodeColumn::Status,
    NodeColumn::Roles,
    NodeColumn::Age,
    NodeColumn::Version,
];

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub enum NodeColumn {
    Name,
    Status,
    Roles,
    Age,
    Version,
    InternalIP,
    ExternalIP,
    OSImage,
    KernelVersion,
    ContainerRuntime,
}

#[allow(dead_code)]
impl NodeColumn {
    /// Must match the Table API columnDefinitions[].name.
    pub const fn as_str(&self) -> &'static str {
        match self {
            NodeColumn::Name => "Name",
            NodeColumn::Status => "Status",
            NodeColumn::Roles => "Roles",
            NodeColumn::Age => "Age",
            NodeColumn::Version => "Version",
            NodeColumn::InternalIP => "Internal-IP",
            NodeColumn::ExternalIP => "External-IP",
            NodeColumn::OSImage => "OS-Image",
            NodeColumn::KernelVersion => "Kernel-Version",
            NodeColumn::ContainerRuntime => "Container-Runtime",
        }
    }

    pub const fn display(&self) -> &'static str {
        match self {
            NodeColumn::Name => "NAME",
            NodeColumn::Status => "STATUS",
            NodeColumn::Roles => "ROLES",
            NodeColumn::Age => "AGE",
            NodeColumn::Version => "VERSION",
            NodeColumn::InternalIP => "INTERNAL-IP",
            NodeColumn::ExternalIP => "EXTERNAL-IP",
            NodeColumn::OSImage => "OS-IMAGE",
            NodeColumn::KernelVersion => "KERNEL-VERSION",
            NodeColumn::ContainerRuntime => "CONTAINER-RUNTIME",
        }
    }

    pub fn normalize_column(column: &str) -> String {
        column.to_lowercase().replace([' ', '_', '-'], "")
    }

    /// All builtin columns.
    pub fn all() -> impl Iterator<Item = NodeColumn> {
        NodeColumn::iter()
    }
}

#[derive(Debug)]
pub struct NodeColumnParseError;

impl std::fmt::Display for NodeColumnParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid NodeColumn string representation")
    }
}

impl std::error::Error for NodeColumnParseError {}

impl std::str::FromStr for NodeColumn {
    type Err = NodeColumnParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::normalize_column(s).as_str() {
            "name" => Ok(NodeColumn::Name),
            "status" => Ok(NodeColumn::Status),
            "roles" => Ok(NodeColumn::Roles),
            "age" => Ok(NodeColumn::Age),
            "version" => Ok(NodeColumn::Version),
            "internalip" => Ok(NodeColumn::InternalIP),
            "externalip" => Ok(NodeColumn::ExternalIP),
            "osimage" => Ok(NodeColumn::OSImage),
            "kernelversion" => Ok(NodeColumn::KernelVersion),
            "containerruntime" => Ok(NodeColumn::ContainerRuntime),
            _ => Err(NodeColumnParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::str::FromStr as _;

    fn builtins(cols: &[NodeColumn]) -> Vec<NodeColumnSpec> {
        cols.iter().map(|c| NodeColumnSpec::Builtin(*c)).collect()
    }

    #[test]
    fn default_columns_are_name_status_roles_age_version() {
        let actual = NodeColumns::default();
        let expected = builtins(&[
            NodeColumn::Name,
            NodeColumn::Status,
            NodeColumn::Roles,
            NodeColumn::Age,
            NodeColumn::Version,
        ]);
        assert_eq!(actual.specs(), expected.as_slice());
    }

    #[test]
    fn builtin_spec_header_is_uppercase_display() {
        assert_eq!(
            NodeColumnSpec::Builtin(NodeColumn::Status).header(),
            "STATUS"
        );
    }

    #[test]
    fn label_spec_header_is_as_given() {
        let s = NodeColumnSpec::Label {
            key: "x".into(),
            header: "MIG".into(),
        };
        assert_eq!(s.header(), "MIG");
    }

    #[test]
    fn from_str_normalizes_case_and_separators() {
        assert_eq!(
            NodeColumn::from_str("internal-ip").unwrap(),
            NodeColumn::InternalIP
        );
        assert_eq!(
            NodeColumn::from_str("OS_Image").unwrap(),
            NodeColumn::OSImage
        );
        assert_eq!(
            NodeColumn::from_str(" Version ").unwrap(),
            NodeColumn::Version
        );
        assert!(NodeColumn::from_str("bogus").is_err());
    }

    #[test]
    fn as_str_matches_table_column_definition_names() {
        assert_eq!(NodeColumn::InternalIP.as_str(), "Internal-IP");
        assert_eq!(NodeColumn::ContainerRuntime.as_str(), "Container-Runtime");
        assert_eq!(NodeColumn::Roles.as_str(), "Roles");
    }

    #[test]
    fn ensure_name_column_prepends_name_when_missing() {
        let cols = NodeColumns::from_builtins([NodeColumn::Status]).ensure_name_column();
        assert_eq!(
            cols.specs(),
            builtins(&[NodeColumn::Name, NodeColumn::Status]).as_slice()
        );
    }
}
