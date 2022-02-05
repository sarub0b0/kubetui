use std::cell::RefCell;
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Paragraph},
    Frame,
};

use derivative::*;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::event::UserEvent;
use crate::{
    clipboard_wrapper::{ClipboardContextWrapper, ClipboardProvider},
    tui_wrapper::event::Callback,
};

use super::{
    config::WidgetConfig,
    RenderTrait, {Item, WidgetTrait},
};

use super::super::{
    event::{EventResult, InnerCallback},
    key_event_to_code, Window,
};

const SCROLL_SIZE: usize = 10;

mod inner_item {
    use super::super::{spans::generate_spans, wrap::*, Item};
    use tui::text::Spans;

    #[derive(Debug, Default)]
    pub struct InnerItem<'a> {
        items: Vec<String>,
        spans: Vec<Spans<'a>>,
        max_width: usize,
    }

    impl<'a> InnerItem<'a> {
        pub fn is_empty(&self) -> bool {
            self.items.is_empty()
        }

        pub fn items(&self) -> &Vec<String> {
            &self.items
        }

        pub fn spans(&self) -> &Vec<Spans<'a>> {
            &self.spans
        }

        #[allow(dead_code)]
        pub fn items_mut(&mut self) -> &mut Vec<String> {
            &mut self.items
        }

        pub fn spans_mut(&mut self) -> &mut Vec<Spans<'a>> {
            &mut self.spans
        }

        pub fn update_max_width(&mut self, max_width: usize) {
            self.max_width = max_width;
            self.update_spans();
        }

        pub fn update_item(&mut self, item: Item) {
            self.items = item.array();

            self.update_spans();
        }

        pub fn update_spans(&mut self) {
            let lines = wrap(&self.items, self.max_width);

            self.spans = generate_spans(&lines);
        }

        pub fn append_widget_item(&mut self, item: Item) {
            let mut item = item.array();

            let wrapped = wrap(&item, self.max_width);

            self.items.append(&mut item);

            self.spans.append(&mut generate_spans(&wrapped));
        }
    }
}

mod highlight_content {
    use super::TextState;
    use tui::text::Spans;

    #[derive(Debug, PartialEq)]
    pub enum RangeType {
        Full,
        StartLine(usize),
        EndLine(usize),
        Partial(usize, usize),
    }

    #[derive(Default, Debug, Copy, Clone)]
    pub struct HighlightArea {
        start: (usize, usize),
        end: (usize, usize),
    }

    impl HighlightArea {
        pub fn start(mut self, start: (usize, usize)) -> Self {
            self.start = start;
            self
        }

        pub fn end(mut self, end: (usize, usize)) -> Self {
            self.end = end;
            self
        }

        pub fn update_pos(&mut self, pos: (usize, usize)) {
            self.end = pos;
        }

        pub fn highlight_ranges(&self) -> Vec<(usize, RangeType)> {
            use std::mem::swap;

            let mut area = *self;

            if (area.end.1 < area.start.1)
                || (area.start.1 == area.end.1 && area.end.0 < area.start.0)
            {
                swap(&mut area.start, &mut area.end);
            }

            let start = area.start.1;
            let end = area.end.1;

            let mut ret = Vec::new();
            for i in start..=end {
                match i {
                    i if start == i && end == i => {
                        ret.push((i, RangeType::Partial(area.start.0, area.end.0)));
                    }
                    i if start == i => {
                        ret.push((i, RangeType::StartLine(area.start.0)));
                    }
                    i if end == i => {
                        ret.push((i, RangeType::EndLine(area.end.0)));
                    }
                    _ => {
                        ret.push((i, RangeType::Full));
                    }
                }
            }

            ret
        }
    }

    #[derive(Default, Debug, Clone)]
    pub struct HighlightContent<'a> {
        pub spans: Vec<Spans<'a>>,
        pub area: HighlightArea,
        pub state: TextState,
        pub copy_content: Vec<String>,
        pub follow: bool,
    }

    #[cfg(test)]
    mod highlight_area {

        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn move_up() {
            let mut area = HighlightArea::default().start((10, 10)).end((10, 10));

            area.update_pos((11, 8));

            assert_eq!(
                area.highlight_ranges(),
                vec![
                    (8, RangeType::StartLine(11)),
                    (9, RangeType::Full),
                    (10, RangeType::EndLine(10)),
                ]
            )
        }

        #[test]
        fn move_down() {
            let mut area = HighlightArea::default().start((10, 10)).end((10, 10));

            area.update_pos((10, 12));

            assert_eq!(
                area.highlight_ranges(),
                vec![
                    (10, RangeType::StartLine(10)),
                    (11, RangeType::Full),
                    (12, RangeType::EndLine(10)),
                ]
            )
        }

        #[test]
        fn move_left() {
            let mut area = HighlightArea::default().start((10, 10)).end((10, 10));

            area.update_pos((0, 10));

            assert_eq!(
                area.highlight_ranges(),
                vec![(10, RangeType::Partial(0, 10))]
            )
        }

        #[test]
        fn move_right() {
            let mut area = HighlightArea::default().start((10, 10)).end((10, 10));

            area.update_pos((20, 10));

            assert_eq!(
                area.highlight_ranges(),
                vec![(10, RangeType::Partial(10, 20))]
            )
        }
    }
}

type RenderBlockInjection = Rc<dyn Fn(&Text, bool) -> Block<'static>>;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct TextBuilder {
    id: String,
    widget_config: WidgetConfig,
    items: Vec<String>,
    wrap: bool,
    follow: bool,
    #[derivative(Debug = "ignore")]
    clipboard: Option<Rc<RefCell<ClipboardContextWrapper>>>,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
}

impl TextBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: &WidgetConfig) -> Self {
        self.widget_config = widget_config.clone();
        self
    }

    pub fn items(mut self, items: impl Into<Vec<String>>) -> Self {
        self.items = items.into();
        self
    }

    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn follow(mut self) -> Self {
        self.follow = true;
        self
    }

    pub fn clipboard(mut self, clipboard: Rc<RefCell<ClipboardContextWrapper>>) -> Self {
        self.clipboard = Some(clipboard);
        self
    }

    pub fn action<F, E: Into<UserEvent>>(mut self, ev: E, cb: F) -> Self
    where
        F: Fn(&mut Window) -> EventResult + 'static,
    {
        self.actions.push((ev.into(), Rc::new(cb)));
        self
    }

    pub fn block_injection<F>(mut self, block_injection: F) -> Self
    where
        F: Fn(&Text, bool) -> Block<'static> + 'static,
    {
        self.block_injection = Some(Rc::new(block_injection));
        self
    }

    pub fn build(self) -> Text<'static> {
        let mut text = Text {
            id: self.id,
            widget_config: self.widget_config,
            wrap: self.wrap,
            follow: self.follow,
            clipboard: self.clipboard,
            actions: self.actions,
            block_injection: self.block_injection,
            ..Default::default()
        };

        text.update_widget_item(Item::Array(self.items));
        text.items.update_max_width(text.wrap_width());
        text
    }
}

use highlight_content::*;
use inner_item::InnerItem;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Text<'a> {
    id: String,
    widget_config: WidgetConfig,
    items: InnerItem<'a>,
    chunk_index: usize,
    state: TextState,
    row_size: u64,
    wrap: bool,
    follow: bool,
    chunk: Rect,
    inner_chunk: Rect,
    highlight_content: Option<HighlightContent<'a>>,
    #[derivative(Debug = "ignore")]
    clipboard: Option<Rc<RefCell<ClipboardContextWrapper>>>,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
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

impl Text<'_> {
    pub fn builder() -> TextBuilder {
        TextBuilder::default()
    }

    pub fn state(&self) -> &TextState {
        &self.state
    }

    pub fn items(&self) -> &[String] {
        self.items.items()
    }

    pub fn rows_size(&self) -> u64 {
        self.row_size
    }

    pub fn status(&self) -> Spans {
        Spans::from(format!(
            "{}/{}",
            self.state.selected_vertical(),
            self.row_size
        ))
    }

    fn match_action(&self, ev: UserEvent) -> Option<InnerCallback> {
        self.actions
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb.clone()) } else { None })
    }

    fn wrap_width(&self) -> usize {
        if self.wrap {
            self.inner_chunk.width as usize
        } else {
            usize::MAX
        }
    }

    fn update_rows_size(&mut self) {
        self.row_size = self
            .items
            .spans()
            .len()
            .saturating_sub(self.inner_chunk.height as usize) as u64;
    }

    fn view_range(&self) -> (usize, usize) {
        let start = self.state.selected_vertical() as usize;

        let end = if self.items.spans().len() < self.inner_chunk.height as usize {
            self.items.spans().len()
        } else {
            start + self.inner_chunk.height as usize
        };

        (start, end)
    }

    fn is_bottom(&self) -> bool {
        self.state.selected_vertical() == self.row_size
    }

    fn scroll_left(&mut self, index: u64) {
        if self.wrap {
            return;
        }
        self.state
            .select_horizontal(self.state.selected_horizontal().saturating_sub(index));
    }

    fn scroll_right(&mut self, index: u64) {
        if self.wrap {
            return;
        }
        self.state
            .select_horizontal(self.state.selected_horizontal().saturating_add(index));
    }

    fn update_select(&mut self, is_bottom: bool) {
        if self.follow && is_bottom {
            self.select_last();
        }

        if self.row_size < self.state.selected_vertical() {
            self.select_last();
        }
    }
}

impl WidgetTrait for Text<'_> {
    fn id(&self) -> &str {
        &self.id
    }

    fn focusable(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<Item> {
        let index = self.state.selected_vertical() as usize;
        Some(Item::Single(self.items.items()[index].clone()))
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        todo!()
    }

    fn select_next(&mut self, index: usize) {
        let mut i = self.state.selected_vertical();

        if self.row_size <= i + index as u64 {
            i = self.row_size;
        } else {
            i += index as u64;
        }

        self.state.select_vertical(i);
    }

    fn select_prev(&mut self, index: usize) {
        self.state
            .select_vertical(self.state.selected_vertical().saturating_sub(index as u64));
    }

    fn select_first(&mut self) {
        self.state.select_vertical(0);
    }

    fn select_last(&mut self) {
        self.state.select_vertical(self.row_size);
    }

    fn append_widget_item(&mut self, items: Item) {
        let is_bottom = self.is_bottom();

        self.items.append_widget_item(items);

        self.update_rows_size();

        if self.follow && is_bottom {
            self.select_last()
        }
    }

    fn update_widget_item(&mut self, items: Item) {
        let is_bottom = self.is_bottom();

        self.items.update_item(items);
        self.update_rows_size();

        self.update_select(is_bottom);
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        fn clear_highlight<'a>(dst: &mut [Spans<'a>], src: &[Spans<'a>], len: usize) {
            dst[..len].clone_from_slice(&src[..len])
        }

        if self.items.is_empty() {
            return EventResult::Nop;
        }

        let (mut col, mut row) = (
            ev.column.saturating_sub(self.inner_chunk.left()) as usize,
            ev.row.saturating_sub(self.inner_chunk.top()) as usize
                + self.state.selected_vertical() as usize,
        );

        let spans_len = self.items.spans().len();
        if spans_len <= row {
            row = spans_len.saturating_sub(1);
        }

        let spans_width = self.items.spans()[row].width();
        if spans_width <= col {
            col = spans_width.saturating_sub(1);
        }

        // TODO スクロールを使って画面外の文字列をコピーできるようにする
        let (start, end) = self.view_range();
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let (spans, content) =
                    highlight_content_partial(self.items.spans()[row].clone(), (col, col));

                self.highlight_content = Some(HighlightContent {
                    spans: self.items.spans()[start..end].to_vec(),
                    area: HighlightArea::default().start((col, row)).end((col, row)),
                    state: self.state,
                    copy_content: vec![content],
                    follow: self.follow,
                });

                self.items.spans_mut()[row] = spans;
                self.follow = false;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(highlight_content) = self.highlight_content.as_mut() {
                    clear_highlight(
                        &mut self.items.spans_mut()[start..end],
                        &highlight_content.spans,
                        end.saturating_sub(start),
                    );

                    highlight_content.area.update_pos((col, row));

                    let mut copy_content = Vec::new();
                    for (row, range) in highlight_content.area.highlight_ranges() {
                        let (s, e) = match range {
                            RangeType::Full => {
                                (0, self.items.spans()[row].width().saturating_sub(1))
                            }
                            RangeType::StartLine(i) => {
                                (i, self.items.spans()[row].width().saturating_sub(1))
                            }
                            RangeType::EndLine(i) => (0, i),
                            RangeType::Partial(i, j) => (i, j),
                        };

                        let (spans, content) =
                            highlight_content_partial(self.items.spans()[row].clone(), (s, e));

                        self.items.spans_mut()[row] = spans;

                        copy_content.push(content);
                    }

                    highlight_content.copy_content = copy_content;
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(highlight_content) = self.highlight_content.as_mut() {
                    clear_highlight(
                        &mut self.items.spans_mut()[start..end],
                        &highlight_content.spans,
                        end.saturating_sub(start),
                    );

                    self.follow = highlight_content.follow;

                    if let Some(clipboard) = &self.clipboard {
                        let content = std::mem::take(&mut highlight_content.copy_content);

                        clipboard
                            .borrow_mut()
                            .set_contents(
                                content
                                    .into_iter()
                                    .map(|mut c| {
                                        if c.width() != self.wrap_width() {
                                            c.push('\n');
                                        }
                                        c
                                    })
                                    .collect::<Vec<String>>()
                                    .concat()
                                    .trim_end()
                                    .to_string(),
                            )
                            .unwrap();
                    }
                }

                self.highlight_content = None;
            }
            MouseEventKind::Down(_) => {}
            MouseEventKind::Drag(_) => {}
            MouseEventKind::Up(_) => {}
            MouseEventKind::Moved => {}
            MouseEventKind::ScrollDown => {
                self.select_next(SCROLL_SIZE);
            }
            MouseEventKind::ScrollUp => {
                self.select_prev(SCROLL_SIZE);
            }
        }
        EventResult::Nop
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match key_event_to_code(ev) {
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next(1);
            }

            KeyCode::PageDown => {
                self.select_next(SCROLL_SIZE);
            }

            KeyCode::Char('k') | KeyCode::Up => {
                self.select_prev(1);
            }

            KeyCode::PageUp => {
                self.select_prev(SCROLL_SIZE);
            }

            KeyCode::Char('G') | KeyCode::End => {
                self.select_last();
            }

            KeyCode::Char('g') | KeyCode::Home => {
                self.select_first();
            }

            KeyCode::Left => {
                self.scroll_left(10);
            }

            KeyCode::Right => {
                self.scroll_right(10);
            }

            _ => {
                if let Some(cb) = self.match_action(UserEvent::Key(ev)) {
                    return EventResult::Callback(Some(Callback::from(cb)));
                }
                return EventResult::Ignore;
            }
        }
        EventResult::Nop
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
        self.inner_chunk = self.widget_config.block().inner(chunk);

        let is_bottom = self.is_bottom();

        self.items.update_max_width(self.wrap_width());
        self.update_rows_size();

        self.update_select(is_bottom);
    }

    fn clear(&mut self) {
        self.items = Default::default();
        self.state = TextState::default();
        self.row_size = 0;
        self.items.update_max_width(self.wrap_width());

        *(self.widget_config.append_title_mut()) = None;
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }
}

fn width_base_range_to_graphemes_range(
    content: &[&str],
    (start, end): (usize, usize),
) -> (usize, usize) {
    let mut start_index = None;
    let mut end_index = None;

    let mut sum = 0;
    for (i, g) in content.iter().enumerate() {
        let width = g.width();

        if start_index.is_none() && start < sum + width {
            start_index = Some(i);
        }

        if start_index.is_some() && end < sum + width {
            end_index = Some(i);
            break;
        }

        sum += width;
    }

    let start_index = if let Some(i) = start_index { i } else { 0 };

    let end_index = if let Some(i) = end_index {
        i
    } else {
        content.len()
    };

    (start_index, end_index)
}

fn highlight_content_partial(src: Spans, (start, end): (usize, usize)) -> (Spans, String) {
    let end = end + 1;
    let content: String = src.clone().into();
    let content: Vec<&str> = content.graphemes(true).collect();

    let mut start_index = 0;
    let mut sum_width_0 = 0;
    let mut sum_width_1 = 0;
    let mut end_index = 0;
    let mut spans: Vec<Span> = src
        .0
        .into_iter()
        .enumerate()
        .flat_map(|(i, span)| {
            let width = span.width();
            let ret = if sum_width_0 <= start && start < sum_width_0 + width {
                if sum_width_0 != start {
                    let (s, e) =
                        width_base_range_to_graphemes_range(&content, (sum_width_0, start));

                    let first = content[s..e].concat();

                    let (s, e) =
                        width_base_range_to_graphemes_range(&content, (start, sum_width_0 + width));

                    let second = content[s..e].concat();

                    start_index = i + 1;

                    vec![
                        Span::styled(first, span.style),
                        Span::styled(second, span.style),
                    ]
                } else {
                    start_index = i;
                    vec![span]
                }
            } else {
                vec![span]
            };

            sum_width_0 += width;

            ret
        })
        .enumerate()
        .flat_map(|(i, span)| {
            let width = span.width();
            let ret = if sum_width_1 < end && end <= sum_width_1 + width {
                if sum_width_1 + width != end {
                    let (s, e) = width_base_range_to_graphemes_range(&content, (sum_width_1, end));
                    let first = content[s..e].concat();

                    let (s, e) =
                        width_base_range_to_graphemes_range(&content, (end, sum_width_1 + width));
                    let second = content[s..e].concat();

                    end_index = i;
                    vec![
                        Span::styled(first, span.style),
                        Span::styled(second, span.style),
                    ]
                } else {
                    end_index = i;
                    vec![span]
                }
            } else {
                vec![span]
            };

            sum_width_1 += width;

            ret
        })
        .collect();

    for span in &mut spans[start_index..=end_index] {
        span.style = Style::default().add_modifier(Modifier::REVERSED)
    }

    let (s, e) = width_base_range_to_graphemes_range(&content, (start, end));
    (Spans::from(spans), content[s..e].concat())
}

impl RenderTrait for Text<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        let (start, end) = self.view_range();

        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, selected)
        } else {
            self.widget_config
                .render_block_with_title(self.focusable() && selected)
        };

        let mut widget = Paragraph::new(self.items.spans()[start..end].to_vec())
            .style(Style::default())
            .block(block);

        if !self.wrap {
            widget = widget.scroll((0, self.state.selected_horizontal() as u16));
        }

        f.render_widget(widget, self.chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn disable_wrap() {
        let data = (0..10).map(|_| "abcd\nefg".to_string()).collect();

        let mut text = TextBuilder::default().build();

        text.update_widget_item(Item::Array(data));

        assert_eq!(text.items.spans().len(), 20)
    }

    #[test]
    fn enable_wrap() {
        let data = (0..10).map(|_| "abcd\nefg".to_string()).collect();

        let mut text = TextBuilder::default().wrap().build();

        text.update_chunk(Rect::new(0, 0, 4, 12));

        text.update_widget_item(Item::Array(data));

        assert_eq!(text.items.spans().len(), 40)
    }

    #[test]
    fn append_items_enable_follow_and_wrap() {
        let data: Vec<String> = (0..10).map(|_| "abcd\nefg".to_string()).collect();

        let mut text = TextBuilder::default().wrap().follow().build();

        text.update_chunk(Rect::new(0, 0, 2, 10));
        text.append_widget_item(Item::Array(data));

        assert!(text.is_bottom())
    }

    mod content_highlight {
        use super::*;
        use crate::tui_wrapper::widget::spans::generate_spans_line;
        use pretty_assertions::assert_eq;
        use tui::style::Color;

        mod graphemes {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn ascii_one() {
                let content: Vec<&str> = "abcdefg".graphemes(true).collect();

                assert_eq!(
                    width_base_range_to_graphemes_range(&content, (2, 2)),
                    (2, 2)
                );
            }

            #[test]
            fn ascii_all() {
                let content: Vec<&str> = "abcdefg".graphemes(true).collect();

                assert_eq!(
                    width_base_range_to_graphemes_range(&content, (0, 6)),
                    (0, 6)
                );
            }

            #[test]
            fn ascii() {
                let content: Vec<&str> = "abcdefg".graphemes(true).collect();

                assert_eq!(
                    width_base_range_to_graphemes_range(&content, (2, 5)),
                    (2, 5)
                );
                assert_eq!(
                    width_base_range_to_graphemes_range(&content, (6, 6)),
                    (6, 6)
                );
            }

            #[test]
            fn japanese_fullwidth() {
                let content: Vec<&str> = "アイウエオ".graphemes(true).collect();

                assert_eq!(
                    width_base_range_to_graphemes_range(&content, (2, 5)),
                    (1, 2)
                );
                assert_eq!(
                    width_base_range_to_graphemes_range(&content, (1, 2)),
                    (0, 1)
                );
            }

            #[test]
            fn japanese_halfwidth() {
                let content: Vec<&str> = "ｱｲｳｴｵ".graphemes(true).collect();

                assert_eq!(
                    width_base_range_to_graphemes_range(&content, (2, 3)),
                    (2, 3)
                )
            }

            #[test]
            fn complex() {
                let content: Vec<&str> = "aあbいcdうefｱｲｳｴｵg".graphemes(true).collect();

                assert_eq!(
                    width_base_range_to_graphemes_range(&content, (2, 5)),
                    (1, 3)
                );
            }
        }

        #[test]
        fn line_all() {
            let text = vec![
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/"
                    .to_string(),
            ];

            let spans = generate_spans_line(&text)[0].clone();

            let hi = highlight_content_partial(spans, (0, 48));

            assert_eq!(
                hi.1,
                "ℹ ｢wds｣: Project is running at http://10.1.157.9/".to_string()
            );

            assert_eq!(
                hi.0,
                Spans::from(vec![
                    Span::styled("ℹ", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled("｢wds｣", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled(
                        ": Project is running at http://10.1.157.9/",
                        Style::default().add_modifier(Modifier::REVERSED)
                    )
                ])
            );
        }

        #[test]
        fn line_partial_char() {
            let text = vec![
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/"
                    .to_string(),
            ];

            let spans = generate_spans_line(&text)[0].clone();

            let hi = highlight_content_partial(spans, (0, 0));

            assert_eq!(hi.1, "ℹ".to_string());

            assert_eq!(
                hi.0,
                Spans::from(vec![
                    Span::styled("ℹ", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled(" ", Style::default().fg(Color::Reset)),
                    Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        ": Project is running at http://10.1.157.9/",
                        Style::default().fg(Color::Reset)
                    )
                ])
            );
        }

        #[test]
        fn line_partial_chars() {
            let text = vec![
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/"
                    .to_string(),
            ];

            let spans = generate_spans_line(&text)[0].clone();

            let hi = highlight_content_partial(spans, (0, 1));

            assert_eq!(hi.1, "ℹ ".to_string());

            assert_eq!(
                hi.0,
                Spans::from(vec![
                    Span::styled("ℹ", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        ": Project is running at http://10.1.157.9/",
                        Style::default().fg(Color::Reset)
                    )
                ])
            );
        }

        #[test]
        fn line_partial_left() {
            let text = vec![
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/"
                    .to_string(),
            ];

            let spans = generate_spans_line(&text)[0].clone();

            let hi = highlight_content_partial(spans, (0, 10));

            assert_eq!(hi.1, "ℹ ｢wds｣: Pr".to_string());

            assert_eq!(
                hi.0,
                Spans::from(vec![
                    Span::styled("ℹ", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled("｢wds｣", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled(": Pr", Style::default().add_modifier(Modifier::REVERSED)),
                    Span::styled(
                        "oject is running at http://10.1.157.9/",
                        Style::default().fg(Color::Reset)
                    ),
                ])
            );
        }

        #[test]
        fn line_partial_middle() {
            let text = vec![
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/"
                    .to_string(),
            ];

            let spans = generate_spans_line(&text)[0].clone();

            let hi = highlight_content_partial(spans, (10, 20));

            assert_eq!(hi.1, "roject is r".to_string());

            assert_eq!(
                hi.0,
                Spans::from(vec![
                    Span::styled("ℹ", Style::default().fg(Color::Blue)),
                    Span::styled(" ", Style::default().fg(Color::Reset)),
                    Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                    Span::styled(": P", Style::default().fg(Color::Reset)),
                    Span::styled(
                        "roject is r",
                        Style::default().add_modifier(Modifier::REVERSED)
                    ),
                    Span::styled(
                        "unning at http://10.1.157.9/",
                        Style::default().fg(Color::Reset)
                    ),
                ])
            );
        }

        #[test]
        fn line_partial_right() {
            let text = vec![
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/"
                    .to_string(),
            ];

            let spans = generate_spans_line(&text)[0].clone();

            let hi = highlight_content_partial(spans, (39, 48));

            assert_eq!(hi.1, "0.1.157.9/".to_string());

            assert_eq!(
                hi.0,
                Spans::from(vec![
                    Span::styled("ℹ", Style::default().fg(Color::Blue)),
                    Span::styled(" ", Style::default().fg(Color::Reset)),
                    Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        ": Project is running at http://1",
                        Style::default().fg(Color::Reset)
                    ),
                    Span::styled(
                        "0.1.157.9/",
                        Style::default().add_modifier(Modifier::REVERSED)
                    ),
                ])
            );
        }
    }
}
