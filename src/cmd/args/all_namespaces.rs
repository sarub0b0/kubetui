use clap::ValueEnum;

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum AllNamespaces {
    True,
    False,
}

impl AllNamespaces {
    pub fn to_bool(self) -> bool {
        match self {
            AllNamespaces::True => true,
            AllNamespaces::False => false,
        }
    }
}

impl From<AllNamespaces> for bool {
    fn from(value: AllNamespaces) -> Self {
        match value {
            AllNamespaces::True => true,
            AllNamespaces::False => false,
        }
    }
}

impl std::fmt::Display for AllNamespaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}
