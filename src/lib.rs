use std::fmt::Display;

pub mod action;

pub mod config;

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

<<<<<<< HEAD
        assert_eq!("None".to_string(), ctx.to_string())
=======
        Kube::GetContextsResponse(ctxs) => {
            update_widget_item_for_vec(window, view_id::subwin_ctx, ctxs);
        }

        Kube::YamlAPIsResponse(apis) => {
            update_widget_item_for_vec(window, view_id::subwin_yaml_kind, apis);
        }

        Kube::YamlResourceResponse(resources) => {
            update_widget_item_for_vec(window, view_id::subwin_yaml_name, resources);
        }

        Kube::YamlRawResponse(yaml) => {
            update_widget_item_for_vec(window, view_id::tab_yaml, yaml);
        }
        _ => unreachable!(),
>>>>>>> poc(show-yaml): yamlを表示するためのベース実装。kind選択、リソース選択、yaml取得・表示まで
    }
}
