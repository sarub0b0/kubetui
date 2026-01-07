use clap::ValueEnum;

#[derive(Debug, Default, ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardMode {
    #[default]
    Auto,
    System,
    Osc52,
}

impl std::fmt::Display for ClipboardMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}
