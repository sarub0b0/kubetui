use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigColumnSpec {
    Builtin(ConfigColumn),
    Label { key: String, header: String },
}

impl ConfigColumnSpec {
    pub fn header(&self) -> String {
        match self {
            ConfigColumnSpec::Builtin(c) => c.display().to_string(),
            ConfigColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigColumns {
    columns: Vec<ConfigColumnSpec>,
}

impl Default for ConfigColumns {
    fn default() -> Self {
        ConfigColumns::from_builtins(DEFAULT_CONFIG_COLUMNS.iter().copied())
    }
}

impl ConfigColumns {
    pub fn new(columns: impl IntoIterator<Item = ConfigColumnSpec>) -> Self {
        ConfigColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn from_builtins(columns: impl IntoIterator<Item = ConfigColumn>) -> Self {
        ConfigColumns {
            columns: columns.into_iter().map(ConfigColumnSpec::Builtin).collect(),
        }
    }

    pub fn specs(&self) -> &[ConfigColumnSpec] {
        &self.columns
    }

    /// KIND と NAME が存在しない場合のみ挿入する（KIND を index 0、NAME を
    /// その直後）。既存の列順は保持し、reorder はしない。
    pub fn ensure_required(mut self) -> Self {
        let has_kind = self
            .columns
            .iter()
            .any(|s| matches!(s, ConfigColumnSpec::Builtin(ConfigColumn::Kind)));
        if !has_kind {
            self.columns
                .insert(0, ConfigColumnSpec::Builtin(ConfigColumn::Kind));
        }

        let kind_pos = self
            .columns
            .iter()
            .position(|s| matches!(s, ConfigColumnSpec::Builtin(ConfigColumn::Kind)))
            .expect("Kind just ensured");
        let has_name = self
            .columns
            .iter()
            .any(|s| matches!(s, ConfigColumnSpec::Builtin(ConfigColumn::Name)));
        if !has_name {
            self.columns
                .insert(kind_pos + 1, ConfigColumnSpec::Builtin(ConfigColumn::Name));
        }

        self
    }

    /// 順序を保ちながら重複を排除。
    pub fn dedup_columns(self) -> Self {
        let mut unique: Vec<ConfigColumnSpec> = Vec::new();
        for spec in self.columns {
            if !unique.contains(&spec) {
                unique.push(spec);
            }
        }
        ConfigColumns { columns: unique }
    }
}

pub const DEFAULT_CONFIG_COLUMNS: &[ConfigColumn] = &[
    ConfigColumn::Kind,
    ConfigColumn::Name,
    ConfigColumn::Data,
    ConfigColumn::Age,
];

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub enum ConfigColumn {
    Kind,
    Name,
    Data,
    Age,
}

impl ConfigColumn {
    pub const fn as_str(&self) -> &'static str {
        match self {
            ConfigColumn::Kind => "Kind",
            ConfigColumn::Name => "Name",
            ConfigColumn::Data => "Data",
            ConfigColumn::Age => "Age",
        }
    }

    pub const fn display(&self) -> &'static str {
        match self {
            ConfigColumn::Kind => "KIND",
            ConfigColumn::Name => "NAME",
            ConfigColumn::Data => "DATA",
            ConfigColumn::Age => "AGE",
        }
    }

    pub fn normalize_column(column: &str) -> String {
        column.to_lowercase().replace([' ', '_', '-'], "")
    }
}

#[derive(Debug)]
pub struct ConfigColumnParseError;

impl std::fmt::Display for ConfigColumnParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid ConfigColumn string representation")
    }
}

impl std::error::Error for ConfigColumnParseError {}

impl std::str::FromStr for ConfigColumn {
    type Err = ConfigColumnParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::normalize_column(s).as_str() {
            "kind" => Ok(ConfigColumn::Kind),
            "name" => Ok(ConfigColumn::Name),
            "data" => Ok(ConfigColumn::Data),
            "age" => Ok(ConfigColumn::Age),
            _ => Err(ConfigColumnParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn builtins(cols: &[ConfigColumn]) -> Vec<ConfigColumnSpec> {
        cols.iter()
            .copied()
            .map(ConfigColumnSpec::Builtin)
            .collect()
    }

    #[test]
    fn default_has_kind_name_data_age_in_order() {
        let cols = ConfigColumns::default();
        assert_eq!(
            cols.specs(),
            builtins(&[
                ConfigColumn::Kind,
                ConfigColumn::Name,
                ConfigColumn::Data,
                ConfigColumn::Age,
            ])
            .as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_both_when_absent() {
        let cols =
            ConfigColumns::from_builtins([ConfigColumn::Data, ConfigColumn::Age]).ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[
                ConfigColumn::Kind,
                ConfigColumn::Name,
                ConfigColumn::Data,
                ConfigColumn::Age,
            ])
            .as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_name_after_existing_kind() {
        let cols =
            ConfigColumns::from_builtins([ConfigColumn::Kind, ConfigColumn::Age]).ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[ConfigColumn::Kind, ConfigColumn::Name, ConfigColumn::Age,]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_kind_when_only_name_present() {
        let cols =
            ConfigColumns::from_builtins([ConfigColumn::Name, ConfigColumn::Age]).ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[ConfigColumn::Kind, ConfigColumn::Name, ConfigColumn::Age,]).as_slice()
        );
    }

    #[test]
    fn ensure_required_preserves_order_when_both_present() {
        let cols = ConfigColumns::from_builtins([
            ConfigColumn::Name,
            ConfigColumn::Kind,
            ConfigColumn::Age,
        ])
        .ensure_required();
        // Order preserved — not reordered to canonical.
        assert_eq!(
            cols.specs(),
            builtins(&[ConfigColumn::Name, ConfigColumn::Kind, ConfigColumn::Age,]).as_slice()
        );
    }

    #[test]
    fn dedup_columns_removes_duplicates_preserving_first() {
        let cols = ConfigColumns::new([
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Builtin(ConfigColumn::Age),
        ])
        .dedup_columns();
        assert_eq!(
            cols.specs(),
            builtins(&[ConfigColumn::Kind, ConfigColumn::Name, ConfigColumn::Age,]).as_slice()
        );
    }

    #[test]
    fn builtin_spec_header_is_uppercase_display() {
        assert_eq!(
            ConfigColumnSpec::Builtin(ConfigColumn::Kind).header(),
            "KIND"
        );
    }

    #[test]
    fn label_spec_header_is_as_given() {
        let s = ConfigColumnSpec::Label {
            key: "app.kubernetes.io/version".to_string(),
            header: "VERSION".to_string(),
        };
        assert_eq!(s.header(), "VERSION");
    }

    #[test]
    fn normalize_column_strips_space_underscore_hyphen_and_lowercases() {
        assert_eq!(ConfigColumn::normalize_column("KIND"), "kind");
        assert_eq!(ConfigColumn::normalize_column("config-map"), "configmap");
        assert_eq!(ConfigColumn::normalize_column("data_count"), "datacount");
    }

    #[test]
    fn ensure_required_inserts_both_into_empty() {
        let cols = ConfigColumns::new([]).ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[ConfigColumn::Kind, ConfigColumn::Name]).as_slice()
        );
    }

    #[test]
    fn ensure_required_prepends_to_label_only_input() {
        let label = ConfigColumnSpec::Label {
            key: "app.kubernetes.io/version".to_string(),
            header: "VERSION".to_string(),
        };
        let cols = ConfigColumns::new([label.clone()]).ensure_required();
        assert_eq!(
            cols.specs(),
            &[
                ConfigColumnSpec::Builtin(ConfigColumn::Kind),
                ConfigColumnSpec::Builtin(ConfigColumn::Name),
                label,
            ]
        );
    }

    #[test]
    fn from_str_accepts_normalized_forms() {
        use std::str::FromStr;
        assert!(matches!(
            ConfigColumn::from_str("KIND"),
            Ok(ConfigColumn::Kind)
        ));
        assert!(matches!(
            ConfigColumn::from_str("name"),
            Ok(ConfigColumn::Name)
        ));
        assert!(ConfigColumn::from_str("bogus").is_err());
    }
}
