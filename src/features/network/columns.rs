use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkColumnSpec {
    Builtin(NetworkColumn),
    Label { key: String, header: String },
}

impl NetworkColumnSpec {
    pub fn header(&self) -> String {
        match self {
            NetworkColumnSpec::Builtin(c) => c.display().to_string(),
            NetworkColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkColumns {
    columns: Vec<NetworkColumnSpec>,
}

impl Default for NetworkColumns {
    fn default() -> Self {
        NetworkColumns::from_builtins(DEFAULT_NETWORK_COLUMNS.iter().copied())
    }
}

impl NetworkColumns {
    pub fn new(columns: impl IntoIterator<Item = NetworkColumnSpec>) -> Self {
        NetworkColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn from_builtins(columns: impl IntoIterator<Item = NetworkColumn>) -> Self {
        NetworkColumns {
            columns: columns
                .into_iter()
                .map(NetworkColumnSpec::Builtin)
                .collect(),
        }
    }

    pub fn specs(&self) -> &[NetworkColumnSpec] {
        &self.columns
    }

    /// KIND と NAME が存在しない場合のみ挿入する (KIND を index 0、NAME を
    /// その直後)。既存の列順は保持し、reorder はしない。
    pub fn ensure_required(mut self) -> Self {
        let has_kind = self
            .columns
            .iter()
            .any(|s| matches!(s, NetworkColumnSpec::Builtin(NetworkColumn::Kind)));
        if !has_kind {
            self.columns
                .insert(0, NetworkColumnSpec::Builtin(NetworkColumn::Kind));
        }

        let kind_pos = self
            .columns
            .iter()
            .position(|s| matches!(s, NetworkColumnSpec::Builtin(NetworkColumn::Kind)))
            .expect("Kind just ensured");
        let has_name = self
            .columns
            .iter()
            .any(|s| matches!(s, NetworkColumnSpec::Builtin(NetworkColumn::Name)));
        if !has_name {
            self.columns.insert(
                kind_pos + 1,
                NetworkColumnSpec::Builtin(NetworkColumn::Name),
            );
        }

        self
    }

    pub fn dedup_columns(self) -> Self {
        let mut unique: Vec<NetworkColumnSpec> = Vec::new();
        for spec in self.columns {
            if !unique.contains(&spec) {
                unique.push(spec);
            }
        }
        NetworkColumns { columns: unique }
    }
}

pub const DEFAULT_NETWORK_COLUMNS: &[NetworkColumn] =
    &[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age];

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub enum NetworkColumn {
    Kind,
    Name,
    Age,
}

impl NetworkColumn {
    pub const fn as_str(&self) -> &'static str {
        match self {
            NetworkColumn::Kind => "Kind",
            NetworkColumn::Name => "Name",
            NetworkColumn::Age => "Age",
        }
    }

    pub const fn display(&self) -> &'static str {
        match self {
            NetworkColumn::Kind => "KIND",
            NetworkColumn::Name => "NAME",
            NetworkColumn::Age => "AGE",
        }
    }

    pub fn normalize_column(column: &str) -> String {
        column.to_lowercase().replace([' ', '_', '-'], "")
    }
}

#[derive(Debug)]
pub struct NetworkColumnParseError;

impl std::fmt::Display for NetworkColumnParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid NetworkColumn string representation")
    }
}

impl std::error::Error for NetworkColumnParseError {}

impl std::str::FromStr for NetworkColumn {
    type Err = NetworkColumnParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::normalize_column(s).as_str() {
            "kind" => Ok(NetworkColumn::Kind),
            "name" => Ok(NetworkColumn::Name),
            "age" => Ok(NetworkColumn::Age),
            _ => Err(NetworkColumnParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn builtins(cols: &[NetworkColumn]) -> Vec<NetworkColumnSpec> {
        cols.iter()
            .copied()
            .map(NetworkColumnSpec::Builtin)
            .collect()
    }

    #[test]
    fn default_has_kind_name_age_in_order() {
        let cols = NetworkColumns::default();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_both_when_absent() {
        let cols = NetworkColumns::from_builtins([NetworkColumn::Age]).ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_name_after_existing_kind() {
        let cols = NetworkColumns::from_builtins([NetworkColumn::Kind, NetworkColumn::Age])
            .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_kind_when_only_name_present() {
        let cols = NetworkColumns::from_builtins([NetworkColumn::Name, NetworkColumn::Age])
            .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_preserves_order_when_both_present() {
        let cols = NetworkColumns::from_builtins([
            NetworkColumn::Name,
            NetworkColumn::Kind,
            NetworkColumn::Age,
        ])
        .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Name, NetworkColumn::Kind, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_both_into_empty() {
        let cols = NetworkColumns::new([]).ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name]).as_slice()
        );
    }

    #[test]
    fn ensure_required_prepends_to_label_only_input() {
        let label = NetworkColumnSpec::Label {
            key: "app.kubernetes.io/name".to_string(),
            header: "APP".to_string(),
        };
        let cols = NetworkColumns::new([label.clone()]).ensure_required();
        assert_eq!(
            cols.specs(),
            &[
                NetworkColumnSpec::Builtin(NetworkColumn::Kind),
                NetworkColumnSpec::Builtin(NetworkColumn::Name),
                label,
            ]
        );
    }

    #[test]
    fn dedup_columns_removes_duplicates_preserving_first() {
        let cols = NetworkColumns::new([
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Builtin(NetworkColumn::Age),
        ])
        .dedup_columns();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn builtin_spec_header_is_uppercase_display() {
        assert_eq!(
            NetworkColumnSpec::Builtin(NetworkColumn::Kind).header(),
            "KIND"
        );
    }

    #[test]
    fn label_spec_header_is_as_given() {
        let s = NetworkColumnSpec::Label {
            key: "app.kubernetes.io/name".to_string(),
            header: "APP".to_string(),
        };
        assert_eq!(s.header(), "APP");
    }

    #[test]
    fn normalize_column_strips_space_underscore_hyphen_and_lowercases() {
        assert_eq!(NetworkColumn::normalize_column("KIND"), "kind");
        assert_eq!(
            NetworkColumn::normalize_column("network-policy"),
            "networkpolicy"
        );
        assert_eq!(NetworkColumn::normalize_column("Age_Group"), "agegroup");
    }

    #[test]
    fn from_str_accepts_normalized_forms() {
        use std::str::FromStr;
        assert!(matches!(
            NetworkColumn::from_str("KIND"),
            Ok(NetworkColumn::Kind)
        ));
        assert!(matches!(
            NetworkColumn::from_str("age"),
            Ok(NetworkColumn::Age)
        ));
        assert!(NetworkColumn::from_str("bogus").is_err());
    }
}
