use std::{borrow::Cow, fmt::Display};

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct WidgetConfigBuilder(WidgetConfig);

/// widgets::Block and Title wrapper
#[derive(Debug, PartialEq, Clone)]
pub struct WidgetConfig {
    title: Title,
    append_title: Option<Title>,
    block: Block<'static>,
    can_activate: bool,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            title: Default::default(),
            append_title: Default::default(),
            block: Block::default()
                .border_type(BorderType::Plain)
                .borders(Borders::ALL),
            can_activate: true,
        }
    }
}

/// builder
impl WidgetConfigBuilder {
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

    pub fn build(self) -> WidgetConfig {
        self.0
    }
}

impl WidgetConfig {
    pub fn builder() -> WidgetConfigBuilder {
        WidgetConfigBuilder::default()
    }

    pub fn block(&self) -> &Block {
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
                    span.style = span.style.add_modifier(Modifier::BOLD);
                });
            } else {
                title.insert(0, " ".into());

                title.iter_mut().for_each(|span| {
                    span.style = span.style.fg(Color::DarkGray);
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
                self.block.clone()
            } else if is_mouse_over {
                self.block
                    .clone()
                    .border_style(Style::default().fg(Color::Gray))
            } else {
                self.block
                    .clone()
                    .border_style(Style::default().fg(Color::DarkGray))
            }
        } else {
            self.block.clone()
        };

        let title = self.render_title(is_active);
        if title.is_empty() {
            block
        } else {
            block.title(title)
        }
    }
}

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
        let wc = WidgetConfig::builder()
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
        let wc = WidgetConfig::builder()
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
