use std::{borrow::Cow, fmt::Display};

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

#[derive(Debug, PartialEq, Clone)]
pub struct WidgetTheme {
    base_style: Style,
    title_active_style: Style,
    title_inactive_style: Style,
    border_type: BorderType,
    border_active_style: Style,
    border_mouse_over_style: Style,
    border_inactive_style: Style,
}

impl Default for WidgetTheme {
    fn default() -> Self {
        Self {
            base_style: Style::default(),
            title_active_style: Style::default().add_modifier(Modifier::BOLD),
            title_inactive_style: Style::default().fg(Color::DarkGray),
            border_type: BorderType::Plain,
            border_active_style: Style::default(),
            border_mouse_over_style: Style::default().fg(Color::Gray),
            border_inactive_style: Style::default().fg(Color::DarkGray),
        }
    }
}

impl WidgetTheme {
    pub fn base_style(mut self, style: impl Into<Style>) -> Self {
        self.base_style = style.into();
        self
    }

    pub fn title_active_style(mut self, style: impl Into<Style>) -> Self {
        self.title_active_style = style.into();
        self
    }

    pub fn title_inactive_style(mut self, style: impl Into<Style>) -> Self {
        self.title_inactive_style = style.into();
        self
    }

    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_type = border_type;
        self
    }

    pub fn border_active_style(mut self, style: impl Into<Style>) -> Self {
        self.border_active_style = style.into();
        self
    }

    pub fn border_mouse_over_style(mut self, style: impl Into<Style>) -> Self {
        self.border_mouse_over_style = style.into();
        self
    }

    pub fn border_inactive_style(mut self, style: impl Into<Style>) -> Self {
        self.border_inactive_style = style.into();
        self
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct WidgetBaseBuilder(WidgetBase);

/// widgets::Block and Title wrapper
#[derive(Debug, PartialEq, Clone)]
pub struct WidgetBase {
    title: Title,
    append_title: Option<Title>,
    block: Block<'static>,
    can_activate: bool,
    theme: WidgetTheme,
}

impl Default for WidgetBase {
    fn default() -> Self {
        Self {
            title: Default::default(),
            append_title: Default::default(),
            block: Block::default()
                .border_type(BorderType::Plain)
                .borders(Borders::ALL),
            can_activate: true,
            theme: Default::default(),
        }
    }
}

/// builder
#[allow(dead_code)]
impl WidgetBaseBuilder {
    pub fn title(mut self, title: impl Into<Title>) -> Self {
        self.0.title = title.into();
        self
    }

    pub fn append_title(mut self, append: impl Into<Title>) -> Self {
        self.0.append_title = Some(append.into());
        self
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.0.block = block;
        self
    }

    /// Border style and title style are default style
    pub fn disable_activation(mut self) -> Self {
        self.0.can_activate = false;
        self
    }

    pub fn theme(mut self, theme: WidgetTheme) -> Self {
        self.0.theme = theme;
        self
    }

    pub fn build(self) -> WidgetBase {
        self.0
    }
}

#[allow(dead_code)]
impl WidgetBase {
    pub fn builder() -> WidgetBaseBuilder {
        WidgetBaseBuilder::default()
    }

    pub fn block(&self) -> &Block<'_> {
        &self.block
    }

    pub fn block_mut(&mut self) -> &mut Block<'static> {
        &mut self.block
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn title_mut(&mut self) -> &mut Title {
        &mut self.title
    }

    pub fn append_title(&self) -> &Option<Title> {
        &self.append_title
    }

    pub fn append_title_mut(&mut self) -> &mut Option<Title> {
        &mut self.append_title
    }

    pub fn render_title(&self, is_active: bool) -> Vec<Span<'static>> {
        if self.title.to_string() == "" {
            return Vec::new();
        }

        let mut title = self.title.spans().spans;

        if let Some(append) = &self.append_title {
            title.append(&mut append.spans().spans);
        }

        title.push(" ".into());

        if self.can_activate {
            if is_active {
                title.insert(0, " + ".into());

                title.iter_mut().for_each(|span| {
                    *span = span.clone().style(self.theme.title_active_style);
                });
            } else {
                title.insert(0, " ".into());

                title.iter_mut().for_each(|span| {
                    *span = span.clone().style(self.theme.title_inactive_style);
                });
            }
        } else {
            title.insert(0, " ".into());
        }

        title
    }

    /// Render Block
    ///
    /// Active:   ─ + Title ───  (BOLD)
    /// Inactive: ─── Title ───  (DarkGray: title is Raw)
    pub fn render_block(&self, is_active: bool, is_mouse_over: bool) -> Block<'static> {
        let block = if self.can_activate {
            if is_active {
                self.block
                    .clone()
                    .border_style(self.theme.border_active_style)
            } else if is_mouse_over {
                self.block
                    .clone()
                    .border_style(self.theme.border_mouse_over_style)
            } else {
                self.block
                    .clone()
                    .border_style(self.theme.border_inactive_style)
            }
        } else {
            self.block.clone()
        };

        let block = block
            .border_type(self.theme.border_type)
            .style(self.theme.base_style);

        let title = self.render_title(is_active);

        if title.is_empty() {
            block
        } else {
            block.title(title)
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub enum Title {
    Raw(String),
    Line(Line<'static>),
    Span(Span<'static>),
}

impl Title {
    pub fn spans(&self) -> Line<'static> {
        match self {
            Title::Raw(title) => Line::from(title.to_string()),
            Title::Line(title) => title.clone(),
            Title::Span(title) => Line::from(title.clone()),
        }
    }
}

impl Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Title::Raw(title) => write!(f, "{}", title),
            Title::Line(title) => write!(
                f,
                "{}",
                title
                    .spans
                    .iter()
                    .cloned()
                    .map(|span| span.content)
                    .collect::<Vec<Cow<str>>>()
                    .concat()
            ),
            Title::Span(title) => write!(f, "{}", title.content),
        }
    }
}

impl Default for Title {
    fn default() -> Self {
        Self::Raw(Default::default())
    }
}

impl From<&str> for Title {
    fn from(title: &str) -> Self {
        Self::Raw(title.into())
    }
}

impl From<String> for Title {
    fn from(title: String) -> Self {
        Self::Raw(title)
    }
}

impl From<&String> for Title {
    fn from(title: &String) -> Self {
        Self::Raw(title.to_string())
    }
}

impl From<Span<'static>> for Title {
    fn from(title: Span<'static>) -> Self {
        Self::Line(title.into())
    }
}

impl From<Line<'static>> for Title {
    fn from(title: Line<'static>) -> Self {
        Self::Line(title)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn render_title() {
        let wc = WidgetBase::builder()
            .title("Title")
            .disable_activation()
            .build();

        let title = wc.render_title(false);

        assert_eq!(
            vec![Span::raw(" "), Span::raw("Title"), Span::raw(" "),],
            title
        )
    }

    #[test]
    fn render_title_with_append() {
        let wc = WidgetBase::builder()
            .title("Title")
            .append_title(" append")
            .disable_activation()
            .build();

        let title = wc.render_title(false);

        assert_eq!(
            vec![
                Span::raw(" "),
                Span::raw("Title"),
                Span::raw(" append"),
                Span::raw(" "),
            ],
            title
        )
    }
}
