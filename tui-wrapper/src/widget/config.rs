use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders},
};

#[derive(Debug, PartialEq, Clone)]
pub struct WidgetConfigBuilder(WidgetConfig);

/// widgets::Block and Title wrapper
#[derive(Debug, PartialEq, Clone)]
pub struct WidgetConfig {
    title: Title,
    append_title: Option<Title>,
    divider: Span<'static>,
    block: Block<'static>,
    focusable: bool,
}

impl Default for WidgetConfigBuilder {
    fn default() -> Self {
        Self(WidgetConfig::default())
    }
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            title: Default::default(),
            append_title: Default::default(),
            divider: Span::raw(": "),
            block: Block::default()
                .border_type(BorderType::Plain)
                .borders(Borders::ALL),
            focusable: true,
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

    pub fn divider(mut self, divider: impl Into<Span<'static>>) -> Self {
        self.0.divider = divider.into();
        self
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.0.block = block;
        self
    }

    /// Border style and title style are default style
    pub fn disable_focus(mut self) -> Self {
        self.0.focusable = false;
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

    /// Render Block
    ///
    /// Focus:     ─ + Title ───  (BOLD)
    /// Not focus: ─── Title ───  (DarkGray: title is Raw)
    pub fn render_block(&self, focused: bool) -> Block {
        self.render_block_(focused)
            .title(self.render_title_(focused))
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

    fn render_title_(&self, focused: bool) -> Vec<Span> {
        let mut title = self.title.spans().0;

        if let Some(append) = &self.append_title {
            title.push(self.divider.clone());

            title.append(&mut append.spans().0);
        }

        title.push(" ".into());

        if self.focusable {
            if focused {
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

    fn render_block_(&self, focused: bool) -> Block {
        if self.focusable {
            if focused {
                self.block.clone().title_offset(1)
            } else {
                self.block
                    .clone()
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title_offset(3)
            }
        } else {
            self.block.clone().title_offset(3)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Title {
    Raw(String),
    Spans(Spans<'static>),
    Span(Span<'static>),
}

impl Title {
    pub fn spans(&self) -> Spans<'static> {
        match self {
            Title::Raw(title) => Spans::from(title.to_string()),
            Title::Spans(title) => title.clone(),
            Title::Span(title) => Spans::from(title.clone()),
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
        Self::Spans(title.into())
    }
}

impl From<Spans<'static>> for Title {
    fn from(title: Spans<'static>) -> Self {
        Self::Spans(title.into())
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
            .disable_focus()
            .build();

        let title = wc.render_title_(false);

        assert_eq!(
            vec![Span::raw(" "), Span::raw("Title"), Span::raw(" "),],
            title
        )
    }

    #[test]
    fn render_title_with_append() {
        let wc = WidgetConfig::builder()
            .title("Title")
            .append_title("append")
            .disable_focus()
            .build();

        let title = wc.render_title_(false);

        assert_eq!(
            vec![
                Span::raw(" "),
                Span::raw("Title"),
                Span::raw(": "),
                Span::raw("append"),
                Span::raw(" "),
            ],
            title
        )
    }
}
