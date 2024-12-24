use std::fmt::Display;

use ratatui::style::Style;

use crate::ui::widget::ansi_color::style_to_ansi;

use super::api_resources::kube::ApiResource;

#[derive(Debug, Clone)]
pub struct StyledApiResource {
    pub resource: ApiResource,
    pub style: Style,
}

impl StyledApiResource {
    pub fn new(resource: ApiResource, style: Style) -> Self {
        Self { resource, style }
    }
}

impl Display for StyledApiResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", style_to_ansi(self.style), self.resource)
    }
}
