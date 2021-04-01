use tui::layout::Rect;
use tui::style::Style;
use tui::text::Spans;
use tui::widgets::{Block, Paragraph};

use super::ansi::*;
use super::WidgetTrait;

const BORDER_WIDTH: usize = 2;

pub struct Text<'a> {
    items: Vec<String>,
    state: TextState,
    spans: Vec<Spans<'a>>,
    row_size: u64,
}

#[derive(Clone, Copy)]
pub struct TextState {
    scroll: u64,
}

impl TextState {
    fn select(&mut self, index: u64) {
        self.scroll = index;
    }
    fn selected(&self) -> u64 {
        self.scroll
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self { scroll: 0 }
    }
}

// ステート
impl Text<'_> {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            state: TextState::default(),
            spans: vec![Spans::default()],
            row_size: 0,
        }
    }

    pub fn select(&mut self, scroll: u64) {
        self.state.select(scroll);
    }

    pub fn state(&self) -> TextState {
        self.state
    }

    pub fn selected(&self) -> u64 {
        self.state.selected()
    }
    pub fn scroll_top(&mut self) {
        self.state.select(0);
    }

    pub fn scroll_bottom(&mut self) {
        self.state.select(self.row_size);
    }

    pub fn is_bottom(&self) -> bool {
        self.selected() == self.row_size
    }

    pub fn scroll_down(&mut self, index: u64) {
        (0..index).for_each(|_| self.select_next(1));
    }

    pub fn scroll_up(&mut self, index: u64) {
        (0..index).for_each(|_| self.select_prev(1));
    }
}

impl Default for Text<'_> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            state: TextState::default(),
            spans: Vec::new(),
            row_size: 0,
        }
    }
}

// コンテンツ操作
impl<'a> Text<'a> {
    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn spans(&self) -> &Vec<Spans> {
        &self.spans
    }

    pub fn widget(&self, block: Block<'a>, area: Rect) -> Paragraph<'a> {
        let area = block.inner(area);

        let start = self.state.selected() as usize;

        let end = if self.spans.len() < area.height as usize {
            self.spans.len()
        } else {
            start + area.height as usize
        };

        Paragraph::new(self.spans[start..end].to_vec())
            .block(block)
            .style(Style::default())
    }

    pub fn row_size(&self) -> u64 {
        self.row_size
    }

    pub fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    pub fn append_items(&mut self, items: &Vec<String>, width: u64, height: u64) {
        self.items.append(&mut items.clone());

        let w = width as usize - BORDER_WIDTH;
        let wrapped = wrap(items, w);

        self.spans.append(&mut generate_spans(&wrapped));

        self.update_rows_size(height);
    }

    pub fn update_spans(&mut self, width: u64) {
        let w = width as usize - BORDER_WIDTH;
        let lines = wrap(&self.items, w);

        self.spans = generate_spans(&lines);
    }

    pub fn update_rows_size(&mut self, height: u64) {
        let mut count = self.spans.len() as u64;

        let height = height - BORDER_WIDTH as u64; // 2: border-line

        if height < count {
            count -= height;
        } else {
            count = 0
        }

        self.row_size = count;
    }
}

impl WidgetTrait for Text<'_> {
    fn selectable(&self) -> bool {
        true
    }

    fn select_next(&mut self, index: usize) {
        let mut i = self.state.selected();

        if self.row_size <= i {
            i = self.row_size;
        } else {
            i = i + index as u64;
        }

        self.state.select(i);
    }

    fn select_prev(&mut self, index: usize) {
        let mut i = self.state.selected();
        if i == 0 {
            i = 0;
        } else {
            i = i - index as u64;
        }
        self.state.select(i);
    }

    fn select_first(&mut self) {
        self.state.select(0);
    }
    fn select_last(&mut self) {
        self.state.select(self.row_size);
    }

    fn set_items(&mut self, items: Vec<String>) {
        self.state.select(0);
        self.items = items;
    }
}
