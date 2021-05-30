use std::cell::RefCell;
use std::rc::Rc;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Paragraph},
    Frame,
};

use clipboard::{ClipboardContext, ClipboardProvider};
use derivative::*;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::RenderTrait;

use super::{WidgetItem, WidgetTrait};

use super::spans::generate_spans;
use super::wrap::*;

#[derive(Debug, PartialEq)]
enum RangeType {
    Full,
    StartLine(usize),
    EndLine(usize),
    Partial(usize, usize),
}

#[derive(Default, Debug, Copy, Clone)]
struct HighlightArea {
    start: (usize, usize),
    end: (usize, usize),
}

impl HighlightArea {
    fn start(mut self, start: (usize, usize)) -> Self {
        self.start = start;
        self
    }

    fn end(mut self, end: (usize, usize)) -> Self {
        self.end = end;
        self
    }

    fn update_pos(&mut self, pos: (usize, usize)) {
        self.end = pos;
    }

    fn highlight_ranges(&self) -> Vec<(usize, RangeType)> {
        use std::mem::swap;

        let mut area = *self;

        if (area.end.1 < area.start.1) || (area.start.1 == area.end.1 && area.end.0 < area.start.0)
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
struct HighlightContent<'a> {
    spans: Vec<Spans<'a>>,
    area: HighlightArea,
    state: TextState,
    copy_content: Vec<String>,
    follow: bool,
}

#[derive(Derivative)]
#[derivative(Debug, Clone, Default)]
pub struct Text<'a> {
    items: Vec<String>,
    state: TextState,
    spans: Vec<Spans<'a>>,
    row_size: u64,
    wrap: bool,
    follow: bool,
    chunk: Rect,
    highlight_content: Option<HighlightContent<'a>>,
    #[derivative(Debug = "ignore")]
    clipboard: Option<Rc<RefCell<ClipboardContext>>>,
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

    pub fn clipboard(mut self, clipboard: Rc<RefCell<ClipboardContext>>) -> Self {
        self.clipboard = Some(clipboard);
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

    pub fn selected_vertical(&self) -> u64 {
        self.state.selected_vertical()
    }

    pub fn selected_horizontal(&self) -> u64 {
        self.state.selected_horizontal()
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

        if self.row_size <= i + index {
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
            self.chunk.width as usize
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

    fn update_spans(&mut self) {
        let lines = wrap(&self.items, self.wrap_width());

        self.spans = generate_spans(&lines);
    }

    fn update_rows_size(&mut self) {
        self.row_size = self
            .spans()
            .len()
            .saturating_sub(self.chunk.height as usize) as u64;
    }

    fn view_range(&self) -> (usize, usize) {
        let start = self.state.selected_vertical() as usize;

        let end = if self.spans.len() < self.chunk.height as usize {
            self.spans.len()
        } else {
            start + self.chunk.height as usize
        };

        (start, end)
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
        let is_bottom = self.is_bottom();
        let pos = self.selected_vertical();

        self.items = items.array();

        self.update_spans();
        self.update_rows_size();

        if self.follow && is_bottom {
            self.scroll_bottom();
        }

        if self.row_size < pos {
            self.scroll_bottom();
        }
    }

    fn update_chunk(&mut self, area: Rect) {
        self.chunk = area;

        let is_bottom = self.is_bottom();
        let pos = self.selected_vertical();

        self.update_spans();
        self.update_rows_size();

        if self.follow && is_bottom {
            self.scroll_bottom();
        }

        if self.row_size < pos {
            self.scroll_bottom();
        }
    }

    fn clear(&mut self) {
        self.items = Vec::default();
        self.spans = Vec::default();
        self.state = TextState::default();
        self.row_size = 0;
    }

    fn get_item(&self) -> Option<WidgetItem> {
        let index = self.state.selected_vertical() as usize;
        Some(WidgetItem::Single(self.spans[index].clone().into()))
    }

    fn append_items(&mut self, items: WidgetItem) {
        let is_bottom = self.is_bottom();

        let items = items.as_array();

        self.items.append(&mut items.to_vec());

        let wrapped = wrap(items, self.wrap_width());

        self.spans.append(&mut generate_spans(&wrapped));

        self.update_rows_size();

        if self.follow && is_bottom {
            self.select_last()
        }
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) {
        fn clear_highlight<'a>(dst: &mut [Spans<'a>], src: &[Spans<'a>], len: usize) {
            for i in 0..len {
                dst[i] = src[i].clone();
            }
        }

        if self.spans.is_empty() || self.clipboard.is_none() {
            return;
        }

        let (mut col, mut row) = (
            ev.column.saturating_sub(self.chunk.left()) as usize,
            ev.row.saturating_sub(self.chunk.top()) as usize
                + self.state.selected_vertical() as usize,
        );

        let spans_len = self.spans.len();
        if spans_len <= row {
            row = self.spans.len().saturating_sub(1);
        }

        let spans_width = self.spans[row].width();
        if spans_width <= col {
            col = spans_width.saturating_sub(1);
        }

        // TODO スクロールを使って画面外の文字列をコピーできるようにする
        let (start, end) = self.view_range();
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let (spans, content) =
                    highlight_content_partial(self.spans[row].clone(), (col, col));

                self.highlight_content = Some(HighlightContent {
                    spans: self.spans[start..end].to_vec(),
                    area: HighlightArea::default().start((col, row)).end((col, row)),
                    state: self.state,
                    copy_content: vec![content],
                    follow: self.follow,
                });

                self.spans[row] = spans;
                self.follow = false;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(highlight_content) = self.highlight_content.as_mut() {
                    clear_highlight(
                        &mut self.spans[start..end],
                        &highlight_content.spans,
                        end.saturating_sub(start),
                    );

                    highlight_content.area.update_pos((col, row));

                    let mut copy_content = Vec::new();
                    for (row, range) in highlight_content.area.highlight_ranges() {
                        let (s, e) = match range {
                            RangeType::Full => (0, self.spans[row].width().saturating_sub(1)),
                            RangeType::StartLine(i) => {
                                (i, self.spans[row].width().saturating_sub(1))
                            }
                            RangeType::EndLine(i) => (0, i),
                            RangeType::Partial(i, j) => (i, j),
                        };

                        let (spans, content) =
                            highlight_content_partial(self.spans[row].clone(), (s, e));

                        self.spans[row] = spans;

                        copy_content.push(content);
                    }

                    highlight_content.copy_content = copy_content;
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(highlight_content) = self.highlight_content.as_mut() {
                    clear_highlight(
                        &mut self.spans[start..end],
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
                                        if c.width() == self.wrap_width() {
                                            c
                                        } else {
                                            c.push('\n');
                                            c
                                        }
                                    })
                                    .collect::<Vec<String>>()
                                    .concat(),
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
            MouseEventKind::ScrollDown => {}
            MouseEventKind::ScrollUp => {}
        }
    }
}

fn highlight_content_partial(src: Spans, (start, end): (usize, usize)) -> (Spans, String) {
    let end = end.saturating_add(1); // カーソル位置を含めるため１を足す
    let content: String = src.clone().into();
    let content: Vec<&str> = content.graphemes(true).collect();

    let mut start_index = 0;
    let mut sum_width = 0;
    let spans: Vec<Span> = src
        .0
        .into_iter()
        .enumerate()
        .flat_map(|(i, span)| {
            let width = span.width();
            let ret = if sum_width <= start && start < sum_width + width {
                if sum_width != start {
                    let first = content[sum_width..start].concat();
                    let second = content[start..(sum_width + width)].concat();

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

            sum_width += width;

            ret
        })
        .collect();

    let mut end_index = 0;
    let mut sum_width = 0;
    let mut spans: Vec<Span> = spans
        .into_iter()
        .enumerate()
        .flat_map(|(i, span)| {
            let width = span.width();
            let ret = if sum_width < end && end <= sum_width + width {
                if sum_width + width != end {
                    let first = content[sum_width..end].concat();
                    let second = content[end..(sum_width + width)].concat();
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

            sum_width += width;

            ret
        })
        .collect();

    for span in &mut spans[start_index..=end_index] {
        span.style = Style::default().add_modifier(Modifier::REVERSED)
    }

    (Spans::from(spans), content[start..end].concat())
}

impl RenderTrait for Text<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, block: Block, chunk: Rect)
    where
        B: Backend,
    {
        let (start, end) = self.view_range();

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

        text.update_chunk(Rect::new(0, 0, 2, 10));
        text.set_items(WidgetItem::Array(data));

        assert_eq!(text.spans().len(), 40)
    }

    #[test]
    fn append_items_enable_follow_and_wrap() {
        let data: Vec<String> = (0..10).map(|_| "abcd\nefg".to_string()).collect();

        let mut text = Text::new(vec![]).enable_wrap().enable_follow();

        text.update_chunk(Rect::new(0, 0, 2, 10));
        text.append_items(WidgetItem::Array(data));

        assert!(text.is_bottom())
    }

    mod highlight {
        use super::*;
        mod content_highlight {
            use super::*;
            use crate::widget::spans::generate_spans_line;
            use pretty_assertions::assert_eq;
            use tui::style::Color;

            #[test]
            fn line_all() {
                let text = vec!["\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/".to_string() ];

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
                let text = vec!["\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/".to_string() ];

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
                let text = vec!["\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/".to_string() ];

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
                let text = vec!["\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/".to_string()];

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
                let text = vec!["\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/".to_string()];

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
                let text = vec!["\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/".to_string()];

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
}
