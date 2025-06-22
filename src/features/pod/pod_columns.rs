use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodColumns {
    columns: Vec<PodColumn>,
}

impl Default for PodColumns {
    fn default() -> Self {
        PodColumns {
            columns: DEFAULT_POD_COLUMNS.to_vec(),
        }
    }
}

impl PodColumns {
    pub fn new(columns: impl IntoIterator<Item = PodColumn>) -> Self {
        PodColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn full() -> Self {
        PodColumns {
            columns: PodColumn::iter().collect(),
        }
    }

    pub fn columns(&self) -> &[PodColumn] {
        &self.columns
    }
}

const DEFAULT_POD_COLUMNS: &[PodColumn] = &[
    PodColumn::Name,
    PodColumn::Ready,
    PodColumn::Status,
    PodColumn::Age,
];

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
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

    mod pod_columns {
        use super::*;

        #[test]
        fn デフォルトのカラムを設定する() {
            let actual = PodColumns::default();
            let expected: Vec<PodColumn> = vec![
                PodColumn::Name,
                PodColumn::Ready,
                PodColumn::Status,
                PodColumn::Age,
            ];
            assert_eq!(actual.columns, expected);
        }

        #[test]
        fn 全カラムを設定する() {
            let actual = PodColumns::full();
            let expected: Vec<PodColumn> = vec![
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

            assert_eq!(actual.columns, expected);
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
