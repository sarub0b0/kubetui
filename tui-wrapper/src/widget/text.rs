use tui::{
    backend::Backend,
    layout::Rect,
    style::Style,
    text::Spans,
    widgets::{Block, Paragraph},
    Frame,
};

use super::RenderTrait;

use super::{WidgetItem, WidgetTrait};

use super::spans::generate_spans;
use super::wrap::*;

#[derive(Debug, Clone, Copy, Default)]
struct TRect {
    width: usize,
    height: usize,
}

impl TRect {
    fn new(rect: Rect) -> Self {
        Self {
            width: rect.width as usize,
            height: rect.height as usize,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Text<'a> {
    items: Vec<String>,
    state: TextState,
    spans: Vec<Spans<'a>>,
    row_size: u64,
    area: TRect,
    wrap: bool,
    follow: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TextState {
    scroll_vertical: u64,
    scroll_horizontal: u64,
}

impl TextState {
    pub fn select_vertical(&mut self, index: u64) {
        self.scroll_vertical = index;
    }

    pub fn select_horizontal(&mut self, index: u64) {
        self.scroll_horizontal = index;
    }

    pub fn selected_vertical(&self) -> u64 {
        self.scroll_vertical
    }

    pub fn selected_horizontal(&self) -> u64 {
        self.scroll_horizontal
    }

    pub fn select(&mut self, index: (u64, u64)) {
        self.scroll_vertical = index.0;
        self.scroll_horizontal = index.1;
    }

    pub fn selected(&self) -> (u64, u64) {
        (self.scroll_vertical, self.scroll_horizontal)
    }
}

// ステート
impl Text<'_> {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            ..Default::default()
        }
    }

    pub fn enable_wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn enable_follow(mut self) -> Self {
        self.follow = true;
        self
    }

    pub fn select_vertical(&mut self, scroll: u64) {
        self.state.select_vertical(scroll);
    }

    pub fn select_horizontal(&mut self, scroll: u64) {
        self.state.select_horizontal(scroll);
    }

    pub fn state(&self) -> &TextState {
        &self.state
    }

    pub fn selected(&self) -> (u64, u64) {
        self.state.selected()
    }
    pub fn scroll_top(&mut self) {
        self.state.select_vertical(0);
    }

    pub fn scroll_bottom(&mut self) {
        self.state.select_vertical(self.row_size);
    }

    pub fn scroll_left(&mut self, index: u64) {
        self.state
            .select_horizontal(self.state.selected_horizontal().saturating_sub(index));
    }

    pub fn scroll_right(&mut self, index: u64) {
        self.state
            .select_horizontal(self.state.selected_horizontal().saturating_add(index));
    }

    pub fn is_bottom(&self) -> bool {
        self.state.selected_vertical() == self.row_size
    }

    pub fn scroll_down(&mut self, index: u64) {
        let mut i = self.state.selected_vertical();

        if self.row_size <= i {
            i = self.row_size;
        } else {
            i += index as u64;
        }

        self.state.select_vertical(i);
    }

    pub fn scroll_up(&mut self, index: u64) {
        self.state
            .select_vertical(self.state.selected_vertical().saturating_sub(index));
    }
}

// コンテンツ操作
impl<'a> Text<'a> {
    fn wrap_width(&self) -> usize {
        if self.wrap {
            self.area.width
        } else {
            usize::MAX
        }
    }

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn spans(&self) -> &Vec<Spans> {
        &self.spans
    }

    pub fn row_size(&self) -> u64 {
        self.row_size
    }

    pub fn append_items(&mut self, items: &[String]) {
        let is_bottom = self.is_bottom();

        self.items.append(&mut items.to_vec());

        let wrapped = wrap(items, self.wrap_width());

        self.spans.append(&mut generate_spans(&wrapped));

        self.update_rows_size();

        if self.follow && is_bottom {
            self.select_last()
        }
    }

    fn update_spans(&mut self) {
        let lines = wrap(&self.items, self.wrap_width());

        self.spans = generate_spans(&lines);
    }

    fn update_rows_size(&mut self) {
        self.row_size = self.spans().len().saturating_sub(self.area.height) as u64;
    }
}

impl WidgetTrait for Text<'_> {
    fn selectable(&self) -> bool {
        true
    }

    fn select_next(&mut self, index: usize) {
        self.scroll_down(index as u64)
    }

    fn select_prev(&mut self, index: usize) {
        self.scroll_up(index as u64)
    }

    fn select_first(&mut self) {
        self.scroll_top();
    }
    fn select_last(&mut self) {
        self.scroll_bottom();
    }

    fn set_items(&mut self, items: WidgetItem) {
        self.state.select_vertical(0);
        self.items = items.array();

        self.update_spans();
        self.update_rows_size();
    }

    fn update_area(&mut self, area: Rect) {
        self.area = TRect::new(area);

        self.update_spans();
        self.update_rows_size();
    }

    fn clear(&mut self) {
        let area = self.area;
        let wrap = self.wrap;
        let follow = self.follow;

        *self = Self::default();

        self.area = area;
        self.wrap = wrap;
        self.follow = follow;
    }

    fn get_item(&self) -> Option<WidgetItem> {
        let index = self.state.selected_vertical() as usize;
        Some(WidgetItem::Single(self.spans[index].clone().into()))
    }
}

impl RenderTrait for Text<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, block: Block, chunk: Rect)
    where
        B: Backend,
    {
        let start = self.state.selected_vertical() as usize;

        let end = if self.spans.len() < self.area.height {
            self.spans.len()
        } else {
            start + self.area.height
        };

        let mut widget = Paragraph::new(self.spans[start..end].to_vec())
            .style(Style::default())
            .block(block);

        if !self.wrap {
            widget = widget.scroll((0, self.state.selected_horizontal() as u16));
        }

        f.render_widget(widget, chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn disable_wrap() {
        let data = (0..10).map(|_| "abcd\nefg".to_string()).collect();

        let mut text = Text::new(vec![]);

        text.set_items(WidgetItem::Array(data));

        assert_eq!(text.spans().len(), 20)
    }

    #[test]
    fn enable_wrap() {
        let data = (0..10).map(|_| "abcd\nefg".to_string()).collect();

        let mut text = Text::new(vec![]).enable_wrap();

        text.update_area(Rect::new(0, 0, 2, 10));
        text.set_items(WidgetItem::Array(data));

        assert_eq!(text.spans().len(), 40)
    }

    #[test]
    fn append_items_enable_follow_and_wrap() {
        let data: Vec<String> = (0..10).map(|_| "abcd\nefg".to_string()).collect();

        let mut text = Text::new(vec![]).enable_wrap().enable_follow();

        text.update_area(Rect::new(0, 0, 2, 10));
        text.append_items(&data);

        assert!(text.is_bottom())
    }
}
