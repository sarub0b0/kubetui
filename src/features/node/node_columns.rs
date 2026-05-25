use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeColumns {
    columns: Vec<NodeColumn>,
}

impl Default for NodeColumns {
    fn default() -> Self {
        NodeColumns {
            columns: DEFAULT_NODE_COLUMNS.to_vec(),
        }
    }
}

#[allow(dead_code)]
impl NodeColumns {
    pub fn new(columns: impl IntoIterator<Item = NodeColumn>) -> Self {
        NodeColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn full() -> Self {
        NodeColumns {
            columns: NodeColumn::iter().collect(),
        }
    }

    pub fn columns(&self) -> &[NodeColumn] {
        &self.columns
    }

    pub fn ensure_name_column(mut self) -> Self {
        if self.columns.contains(&NodeColumn::Name) {
            return self;
        }
        self.columns.insert(0, NodeColumn::Name);
        self
    }

    // Removes duplicates while preserving order.
    // Linear search is used because the number of columns is small.
    #[allow(dead_code)]
    pub fn dedup_columns(self) -> Self {
        let mut unique = Vec::new();
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

    #[allow(dead_code)]
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

    #[test]
    fn default_columns_are_name_status_roles_age_version() {
        let actual = NodeColumns::default();
        let expected = vec![
            NodeColumn::Name,
            NodeColumn::Status,
            NodeColumn::Roles,
            NodeColumn::Age,
            NodeColumn::Version,
        ];
        assert_eq!(actual.columns(), expected.as_slice());
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
        let cols = NodeColumns::new([NodeColumn::Status]).ensure_name_column();
        assert_eq!(cols.columns(), &[NodeColumn::Name, NodeColumn::Status]);
    }
}
