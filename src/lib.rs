use std::fmt::Display;

pub mod action;
pub mod config;
pub mod window;

#[cfg(feature = "logging")]
pub mod log;

#[derive(Debug, Default)]
pub struct Context(pub String);

impl Context {
    pub fn new() -> Self {
        Self("None".to_string())
    }

    pub fn update(&mut self, ctx: impl Into<String>) {
        self.0 = ctx.into();
    }
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default)]
pub struct Namespace {
    pub default: String,
    pub selected: Vec<String>,
}

impl Namespace {
    pub fn new() -> Self {
        Self {
            default: "None".to_string(),
            selected: vec!["None".to_string()],
        }
    }
}

impl Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.selected.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_display() {
        let mut ns = Namespace::new();

        ns.selected = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
        ];

        assert_eq!("a, b, c, d, e".to_string(), ns.to_string())
    }

    #[test]
    fn context_display() {
        let ctx = Context::new();

        assert_eq!("None".to_string(), ctx.to_string())
    }
}
