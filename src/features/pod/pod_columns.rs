use strum::{EnumIter, IntoEnumIterator};

/// A runtime column in the pod table: a built-in column or a label column.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PodColumnSpec {
    Builtin(PodColumn),
    Label { key: String, header: String },
}

impl PodColumnSpec {
    /// Display header (uppercase). Builtin uses display(), Label uses its header.
    pub fn header(&self) -> String {
        match self {
            PodColumnSpec::Builtin(c) => c.display().to_string(),
            PodColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

/// A resolved label-column definition (an entry of the label registry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodColumns {
    columns: Vec<PodColumnSpec>,
}

impl Default for PodColumns {
    fn default() -> Self {
        PodColumns {
            columns: DEFAULT_POD_COLUMNS
                .iter()
                .copied()
                .map(PodColumnSpec::Builtin)
                .collect(),
        }
    }
}

impl PodColumns {
    pub fn new(columns: impl IntoIterator<Item = PodColumnSpec>) -> Self {
        PodColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn from_builtins(columns: impl IntoIterator<Item = PodColumn>) -> Self {
        PodColumns {
            columns: columns.into_iter().map(PodColumnSpec::Builtin).collect(),
        }
    }

    pub fn full() -> Self {
        Self::from_builtins(PodColumn::iter())
    }

    pub fn specs(&self) -> &[PodColumnSpec] {
        &self.columns
    }

    pub fn ensure_name_column(mut self) -> Self {
        let has_name = self
            .columns
            .iter()
            .any(|s| matches!(s, PodColumnSpec::Builtin(PodColumn::Name)));
        if !has_name {
            self.columns
                .insert(0, PodColumnSpec::Builtin(PodColumn::Name));
        }
        self
    }

    // 順序を保ちながら重複を排除します。
    // 列数が少ない前提のため、線形探索を使用しています。
    pub fn dedup_columns(self) -> Self {
        let mut unique: Vec<PodColumnSpec> = Vec::new();
        for c in self.columns {
            if !unique.contains(&c) {
                unique.push(c);
            }
        }
        PodColumns { columns: unique }
    }
}

pub const DEFAULT_POD_COLUMNS: &[PodColumn] = &[
    PodColumn::Name,
    PodColumn::Ready,
    PodColumn::Status,
    PodColumn::Age,
];

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub enum PodColumn {
    Name,
    Ready,
    Status,
    Restarts,
    Age,
    IP,
    Node,
    NominatedNode,
    ReadinessGates,
}

impl PodColumn {
    pub const fn as_str(&self) -> &'static str {
        match self {
            PodColumn::Name => "Name",
            PodColumn::Ready => "Ready",
            PodColumn::Status => "Status",
            PodColumn::Restarts => "Restarts",
            PodColumn::Age => "Age",
            PodColumn::IP => "IP",
            PodColumn::Node => "Node",
            PodColumn::NominatedNode => "Nominated Node",
            PodColumn::ReadinessGates => "Readiness Gates",
        }
    }

    pub const fn normalize(&self) -> &'static str {
        match self {
            PodColumn::Name => "name",
            PodColumn::Ready => "ready",
            PodColumn::Status => "status",
            PodColumn::Restarts => "restarts",
            PodColumn::Age => "age",
            PodColumn::IP => "ip",
            PodColumn::Node => "node",
            PodColumn::NominatedNode => "nominatednode",
            PodColumn::ReadinessGates => "readinessgates",
        }
    }

    pub const fn display(&self) -> &'static str {
        match self {
            PodColumn::Name => "NAME",
            PodColumn::Ready => "READY",
            PodColumn::Status => "STATUS",
            PodColumn::Restarts => "RESTARTS",
            PodColumn::Age => "AGE",
            PodColumn::IP => "IP",
            PodColumn::Node => "NODE",
            PodColumn::NominatedNode => "NOMINATED NODE",
            PodColumn::ReadinessGates => "READINESS GATES",
        }
    }

    pub fn normalize_column(column: &str) -> String {
        column.to_lowercase().replace([' ', '_', '-'], "")
    }
}

#[derive(Debug)]
pub struct PodColumnParseError;

impl std::fmt::Display for PodColumnParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid PodColumn string representation")
    }
}

impl std::error::Error for PodColumnParseError {}

impl std::str::FromStr for PodColumn {
    type Err = PodColumnParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::normalize_column(s).as_str() {
            "name" => Ok(PodColumn::Name),
            "ready" => Ok(PodColumn::Ready),
            "status" => Ok(PodColumn::Status),
            "restarts" => Ok(PodColumn::Restarts),
            "age" => Ok(PodColumn::Age),
            "ip" => Ok(PodColumn::IP),
            "node" => Ok(PodColumn::Node),
            "nominatednode" => Ok(PodColumn::NominatedNode),
            "readinessgates" => Ok(PodColumn::ReadinessGates),
            _ => Err(PodColumnParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtins(cols: &[PodColumn]) -> Vec<PodColumnSpec> {
        cols.iter().map(|c| PodColumnSpec::Builtin(*c)).collect()
    }

    mod pod_columns {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn デフォルトのカラムを設定する() {
            let actual = PodColumns::default();
            let expected = builtins(&[
                PodColumn::Name,
                PodColumn::Ready,
                PodColumn::Status,
                PodColumn::Age,
            ]);
            assert_eq!(actual.specs(), expected.as_slice());
        }

        #[test]
        fn 全カラムを設定する() {
            let actual = PodColumns::full();
            let expected = builtins(&[
                PodColumn::Name,
                PodColumn::Ready,
                PodColumn::Status,
                PodColumn::Restarts,
                PodColumn::Age,
                PodColumn::IP,
                PodColumn::Node,
                PodColumn::NominatedNode,
                PodColumn::ReadinessGates,
            ]);

            assert_eq!(actual.specs(), expected.as_slice());
        }

        mod dedup_columns {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn 重複が排除される() {
                let columns = PodColumns::from_builtins([
                    PodColumn::Ready,
                    PodColumn::Status,
                    PodColumn::Ready,
                    PodColumn::Name,
                ])
                .dedup_columns();

                assert_eq!(
                    columns.specs(),
                    builtins(&[PodColumn::Ready, PodColumn::Status, PodColumn::Name]).as_slice()
                );
            }
        }

        mod ensure_name_column {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn nameカラムがすでに含まれている場合は変更されない() {
                let pod_columns = PodColumns::from_builtins([
                    PodColumn::Name,
                    PodColumn::Ready,
                    PodColumn::Status,
                ]);
                let actual = pod_columns.ensure_name_column();

                assert_eq!(
                    actual.specs(),
                    builtins(&[PodColumn::Name, PodColumn::Ready, PodColumn::Status]).as_slice()
                );
            }

            #[test]
            fn nameカラムが含まれていない場合は先頭に追加される() {
                let pod_columns = PodColumns::from_builtins([PodColumn::Ready, PodColumn::Status]);

                let actual = pod_columns.ensure_name_column();

                assert_eq!(
                    actual.specs(),
                    builtins(&[PodColumn::Name, PodColumn::Ready, PodColumn::Status]).as_slice()
                );
            }
        }
    }

    mod pod_column_spec {
        use super::*;

        #[test]
        fn builtin_spec_header_is_uppercase_display() {
            assert_eq!(PodColumnSpec::Builtin(PodColumn::Status).header(), "STATUS");
        }

        #[test]
        fn label_spec_header_is_as_given() {
            let s = PodColumnSpec::Label {
                key: "app.kubernetes.io/version".to_string(),
                header: "VERSION".to_string(),
            };
            assert_eq!(s.header(), "VERSION");
        }
    }

    mod pod_column {
        use super::*;

        #[test]
        fn 並び替えが定義順と一致する() {
            let mut columns = vec![
                PodColumn::IP,
                PodColumn::Ready,
                PodColumn::Node,
                PodColumn::Name,
                PodColumn::Age,
                PodColumn::Restarts,
                PodColumn::ReadinessGates,
                PodColumn::Status,
                PodColumn::NominatedNode,
            ];

            columns.sort();

            let expected = vec![
                PodColumn::Name,
                PodColumn::Ready,
                PodColumn::Status,
                PodColumn::Restarts,
                PodColumn::Age,
                PodColumn::IP,
                PodColumn::Node,
                PodColumn::NominatedNode,
                PodColumn::ReadinessGates,
            ];

            assert_eq!(columns, expected);
        }

        mod normalize_column {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn 空白を削除して小文字に変換する() {
                let name = "  Name  ";
                let actual = PodColumn::normalize_column(name);
                assert_eq!(actual, "name");
            }

            #[test]
            fn アンダースコアを削除して小文字に変換する() {
                let name = "Nominated_Node";
                let actual = PodColumn::normalize_column(name);
                assert_eq!(actual, "nominatednode");
            }

            #[test]
            fn ハイフンを削除して小文字に変換する() {
                let name = "Readiness-Gates";
                let actual = PodColumn::normalize_column(name);
                assert_eq!(actual, "readinessgates");
            }
        }
    }
}
