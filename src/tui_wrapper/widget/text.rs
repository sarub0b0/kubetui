mod item;
mod render;
mod styled_graphemes;
mod wrap;

use std::{cell::RefCell, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use derivative::Derivative;

use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Paragraph, Widget},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::{
    clipboard_wrapper::ClipboardContextWrapper,
    event::UserEvent,
    tui_wrapper::{
        event::{Callback, EventResult, InnerCallback},
        key_event_to_code, Window,
    },
};

use self::{
    item::TextItem,
    render::{Render, Scroll},
    styled_graphemes::StyledGrapheme,
};

use super::{
    config::WidgetConfig, InputForm, Item, LiteralItem, RenderTrait, SelectedItem, WidgetTrait,
};

type RenderBlockInjection = Rc<dyn Fn(&Text, bool) -> Block<'static>>;

mod highlight_content {
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

#[derive(Debug)]
struct SearchForm {
    input_widget: InputForm,
    chunk: Rect,
}

impl Default for SearchForm {
    fn default() -> Self {
        Self {
            input_widget: InputForm::new(WidgetConfig::builder().block(Block::default()).build()),
            chunk: Default::default(),
        }
    }
}

impl SearchForm {
    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = Rect::new(chunk.x, chunk.y + chunk.height - 1, chunk.width, 1);
    }

    fn word(&self) -> String {
        self.input_widget.content()
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.input_widget.on_key_event(ev)
    }

    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool, status: (usize, usize))
    where
        B: Backend,
    {
        let header = "Search: ";

        let content = self.input_widget.render_content(selected);

        let status = format!(" [{}/{}]", status.0, status.1);

        let content_width = self.chunk.width.saturating_sub(8 + status.width() as u16);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(content_width),
                Constraint::Length(status.len() as u16),
            ])
            .split(self.chunk);

        f.render_widget(Paragraph::new(header), chunks[0]);

        f.render_widget(Paragraph::new(content), chunks[1]);

        f.render_widget(Paragraph::new(status), chunks[2]);
    }
}

#[derive(Debug)]
enum Mode {
    /// 通常 （検索フォーム非表示）
    Normal,
    /// 検索ワード入力中（検索フォーム表示）
    SearchInput,
    /// 検索ワード確定後（検索フォーム表示）
    SearchConfirm,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Normal
    }
}

impl Mode {
    fn normal(&mut self) {
        *self = Mode::Normal;
    }

    fn search_input(&mut self) {
        *self = Mode::SearchInput;
    }

    fn search_confirm(&mut self) {
        *self = Mode::SearchConfirm;
    }

    fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }

    fn is_search_input(&self) -> bool {
        matches!(self, Self::SearchInput)
    }

    fn is_search_confirm(&self) -> bool {
        matches!(self, Self::SearchConfirm)
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct TextBuilder {
    id: String,
    widget_config: WidgetConfig,
    item: Vec<LiteralItem>,
    wrap: bool,
    follow: bool,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    clipboard: Option<Rc<RefCell<ClipboardContextWrapper>>>,
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

    pub fn items(mut self, item: impl Into<Vec<String>>) -> Self {
        let item = item.into();
        self.item = item
            .into_iter()
            .map(|i| LiteralItem::new(i, None))
            .collect();
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

    pub fn clipboard(mut self, clipboard: Rc<RefCell<ClipboardContextWrapper>>) -> Self {
        self.clipboard = Some(clipboard);
        self
    }

    pub fn build(self) -> Text {
        Text {
            id: self.id,
            widget_config: self.widget_config,
            item: TextItem::new(self.item, None),
            wrap: self.wrap,
            follow: self.follow,
            actions: self.actions,
            block_injection: self.block_injection,
            clipboard: self.clipboard,
            ..Default::default()
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Text {
    id: String,
    widget_config: WidgetConfig,
    item: TextItem,
    chunk: Rect,
    wrap: bool,
    follow: bool,
    scroll: Scroll,
    search_widget: SearchForm,
    /// 検索中、検索ワード入力中、オフの3つのモード
    mode: Mode,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    clipboard: Option<Rc<RefCell<ClipboardContextWrapper>>>,
}

impl Text {
    pub fn builder() -> TextBuilder {
        Default::default()
    }
}

/// ワード検索機能
///
/// # Features
///
/// - マッチした文字列をハイライト
/// - マッチした文字列に移動
/// - 検索モード終了時にハイライトを削除
impl Text {
    pub fn search(&mut self) {
        self.mode.search_input();

        let word = self.search_widget.word();

        if word.is_empty() {
            // 入力文字が空の時に1文字だけハイライトが残るのを防ぐため
            self.item.clear_highlight();
            return;
        }

        self.item.highlight(&word);

        if let Some(index) = self.item.select_nearest_highlight(self.scroll.y) {
            self.scroll.y = self.search_scroll(index);
        }
    }

    pub fn search_next(&mut self) {
        if let Some(index) = self.item.select_next_highlight() {
            self.scroll.y = self.search_scroll(index);
        }
    }

    pub fn search_prev(&mut self) {
        if let Some(index) = self.item.select_prev_highlight() {
            self.scroll.y = self.search_scroll(index);
        }
    }

    pub fn search_cancel(&mut self) {
        self.mode.normal();
        self.item.clear_highlight();
    }

    /// 移動したいハイライトが中央になるスクロール位置を返す
    fn search_scroll(&self, search_index: usize) -> usize {
        search_index
            .saturating_sub((self.inner_chunk().height / 2) as usize)
            .min(self.scroll_y_last_index())
    }
}

impl Text {
    pub fn scroll_right(&mut self, i: usize) {
        if self.wrap {
            return;
        }

        self.scroll.x = self.scroll.x.saturating_add(i);
    }

    pub fn scroll_left(&mut self, i: usize) {
        if self.wrap {
            return;
        }

        self.scroll.x = self.scroll.x.saturating_sub(i);
    }

    pub fn scroll_y_last_index(&self) -> usize {
        self.item
            .wrapped()
            .len()
            .saturating_sub(self.inner_chunk().height as usize)
    }

    pub fn chunk(&self) -> Rect {
        let Rect {
            x,
            y,
            width,
            height,
        } = self.chunk;

        if self.mode.is_normal() {
            self.chunk
        } else {
            Rect::new(x, y, width, height.saturating_sub(1))
        }
    }

    pub fn inner_chunk(&self) -> Rect {
        let chunk = self.chunk();

        self.widget_config.block().inner(chunk)
    }
}

impl Text {
    pub fn state(&self) -> (usize, usize) {
        (self.scroll.y, self.scroll_y_last_index())
    }

    fn match_action(&self, ev: UserEvent) -> Option<InnerCallback> {
        self.actions
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb.clone()) } else { None })
    }
}

impl<'a> WidgetTrait for Text {
    fn id(&self) -> &str {
        &self.id
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }

    fn focusable(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<SelectedItem> {
        todo!()
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        todo!()
    }

    fn select_next(&mut self, i: usize) {
        self.scroll.y = self
            .scroll
            .y
            .saturating_add(i)
            .min(self.scroll_y_last_index());
    }

    fn select_prev(&mut self, i: usize) {
        self.scroll.y = self.scroll.y.saturating_sub(i)
    }

    fn select_first(&mut self) {
        self.scroll.y = 0;
    }

    fn select_last(&mut self) {
        self.scroll.y = self.scroll_y_last_index();
    }

    fn append_widget_item(&mut self, item: Item) {
        match item {
            Item::Single(i) => self.item.push(i),
            Item::Array(i) => self.item.extend(i),
            _ => {
                unreachable!()
            }
        }
    }

    fn update_widget_item(&mut self, item: Item) {
        let item = item.array();
        self.item.update(item);
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {}
            MouseEventKind::Drag(MouseButton::Left) => {}
            MouseEventKind::Up(MouseButton::Left) => {}
            MouseEventKind::ScrollDown => {
                self.select_next(5);
            }
            MouseEventKind::ScrollUp => {
                self.select_prev(5);
            }
            _ => {}
        }

        EventResult::Nop
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        use KeyCode::*;

        match self.mode {
            Mode::Normal | Mode::SearchConfirm => match key_event_to_code(ev) {
                Char('j') | Down => {
                    self.select_next(1);
                }

                Char('k') | Up => {
                    self.select_prev(1);
                }

                PageDown => {
                    self.select_next(self.chunk.height as usize);
                }

                PageUp => {
                    self.select_prev(self.chunk.height as usize);
                }

                Char('G') | End => {
                    self.select_last();
                }

                Char('g') | Home => {
                    self.select_first();
                }

                Left => {
                    self.scroll_left(1);
                }

                Right => {
                    self.scroll_right(1);
                }

                Char('/') => {
                    self.search();
                }

                Char('q') | Esc if self.mode.is_search_confirm() => {
                    self.search_cancel();
                }

                Char('n') => {
                    self.search_next();
                }

                Char('N') => {
                    self.search_prev();
                }

                _ => {
                    if let Some(cb) = self.match_action(UserEvent::Key(ev)) {
                        return EventResult::Callback(Some(Callback::from(cb)));
                    }
                    return EventResult::Ignore;
                }
            },

            Mode::SearchInput => match key_event_to_code(ev) {
                Enter => {
                    self.mode.search_confirm();
                }

                Esc => {
                    self.search_cancel();
                }

                _ => {
                    let ev = self.search_widget.on_key_event(ev);

                    self.search();

                    return ev;
                }
            },
        }

        EventResult::Nop
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        if self.wrap {
            self.item.rewrap(self.inner_chunk().width as usize);
        };

        self.search_widget.update_chunk(chunk);
    }

    fn clear(&mut self) {
        self.scroll = Default::default();

        let wrap_width = if self.wrap {
            Some(self.inner_chunk().width as usize)
        } else {
            None
        };

        self.item = TextItem::new(vec![], wrap_width);

        *(self.widget_config.append_title_mut()) = None;
    }
}

impl RenderTrait for Text {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, self.focusable() && selected)
        } else {
            self.widget_config
                .render_block(self.focusable() && selected)
        };

        let wrapped_lines = self.item.wrapped();

        let lines: Vec<&[StyledGrapheme]> =
            wrapped_lines.iter().map(|wrapped| wrapped.line()).collect();

        let r = Render::builder()
            .block(block)
            .lines(&lines)
            .scroll(self.scroll)
            .build();

        match self.mode {
            Mode::Normal => {
                f.render_widget(r, self.chunk());
            }

            Mode::SearchInput | Mode::SearchConfirm => {
                f.render_widget(r, self.chunk());

                self.search_widget.render(
                    f,
                    self.mode.is_search_input(),
                    self.item.highlight_status(),
                );
            }
        }
    }
}
