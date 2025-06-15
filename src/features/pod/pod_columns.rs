use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodColumns {
    pub columns: Vec<PodColumn>,
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

    pub fn contains(&self, column: &PodColumn) -> bool {
        self.columns.contains(column)
    }
}

const DEFAULT_POD_COLUMNS: &[PodColumn] = &[
    PodColumn::Name,
    PodColumn::Ready,
    PodColumn::Status,
    PodColumn::Age,
];

#[derive(EnumIter, PartialEq, Eq, Debug, Clone, Copy)]
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

    pub fn from_str(column: &str) -> Option<Self> {
        match Self::normalize_column(column).as_str() {
            "name" => Some(PodColumn::Name),
            "ready" => Some(PodColumn::Ready),
            "status" => Some(PodColumn::Status),
            "restarts" => Some(PodColumn::Restarts),
            "age" => Some(PodColumn::Age),
            "ip" => Some(PodColumn::IP),
            "node" => Some(PodColumn::Node),
            "nominatednode" => Some(PodColumn::NominatedNode),
            "readinessgates" => Some(PodColumn::ReadinessGates),
            _ => None,
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
    }

    mod pod_column {
        use super::*;

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
