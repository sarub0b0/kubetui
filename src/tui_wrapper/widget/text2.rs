mod item;
mod render;
mod styled_graphemes;
mod wrap;

use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use derivative::*;
use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::StyledGrapheme,
    widgets::{Block, Paragraph, Widget},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::{
    event::UserEvent,
    tui_wrapper::{
        event::{Callback, EventResult, InnerCallback},
        key_event_to_code, Window,
    },
};

use self::{
    item::TextItem,
    render::{Render, Scroll},
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
struct SearchForm<'a> {
    input_widget: InputForm<'a>,
    chunk: Rect,
}

impl Default for SearchForm<'_> {
    fn default() -> Self {
        Self {
            input_widget: InputForm::new(WidgetConfig::builder().block(Block::default()).build()),
            chunk: Default::default(),
        }
    }
}

impl<'a> SearchForm<'a> {
    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
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
        let Rect {
            x,
            y: _,
            width,
            height,
        } = self.chunk;

        let header = "Search: ";

        let content = self.input_widget.render_content(selected);

        let status = format!(" [{}/{}]", status.0, status.1);

        let content_width = width.saturating_sub(8 + status.width() as u16);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(content_width),
                Constraint::Length(status.len() as u16),
            ])
            .split(Rect::new(x, height, width, 1));

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
        match self {
            Self::Normal => true,
            _ => false,
        }
    }

    fn is_search_input(&self) -> bool {
        match self {
            Self::SearchInput => true,
            _ => false,
        }
    }

    fn is_search_confirm(&self) -> bool {
        match self {
            Self::SearchConfirm => true,
            _ => false,
        }
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
}

impl TextBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: WidgetConfig) -> Self {
        self.widget_config = widget_config;
        self
    }

    pub fn item(mut self, item: impl Into<Vec<LiteralItem>>) -> Self {
        self.item = item.into();
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

    pub fn build(self) -> Text<'static> {
        let ret = Text {
            id: self.id,
            widget_config: self.widget_config,
            item: TextItem::new(self.item, None),
            wrap: self.wrap,
            follow: self.follow,
            actions: self.actions,
            block_injection: self.block_injection,
            ..Default::default()
        };

        ret
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Text<'a> {
    id: String,
    widget_config: WidgetConfig,
    item: TextItem<'a>,
    chunk: Rect,
    inner_chunk: Rect,
    wrap: bool,
    follow: bool,
    scroll: Scroll,
    search_widget: SearchForm<'a>,
    /// 検索中、検索ワード入力中、オフの3つのモード
    mode: Mode,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, InnerCallback)>,
}

impl Text<'_> {
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
impl Text<'_> {
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
            .saturating_sub((self.inner_chunk.height / 2) as usize)
            .min(self.scroll_y_last_index())
    }
}

impl Text<'_> {
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
            .saturating_sub(self.inner_chunk.height as usize)
    }
}

impl Text<'_> {
    fn match_action(&self, ev: UserEvent) -> Option<InnerCallback> {
        self.actions
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb.clone()) } else { None })
    }
}

impl<'a> WidgetTrait for Text<'_> {
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

    fn append_widget_item(&mut self, _: Item) {
        todo!()
    }

    fn update_widget_item(&mut self, _: Item) {
        todo!()
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
        let chunk = if self.mode.is_normal() {
            chunk
        } else {
            Rect::new(
                chunk.x,
                chunk.y,
                chunk.width,
                chunk.height.saturating_sub(1),
            )
        };

        self.chunk = chunk;
        self.inner_chunk = self.widget_config.block().inner(chunk);

        if self.wrap {
            self.item.rewrap(self.inner_chunk.width as usize);
        };

        self.search_widget.update_chunk(chunk);
    }

    fn clear(&mut self) {
        todo!()
    }
}

impl RenderTrait for Text<'_> {
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

        let lines: Vec<&[StyledGrapheme<'_>]> = wrapped_lines
            .iter()
            .map(|wrapped| wrapped.line.as_ref())
            .collect();

        let r = Render::builder()
            .block(block)
            .lines(&lines)
            .scroll(self.scroll)
            .build();

        match self.mode {
            Mode::Normal => {
                f.render_widget(r, self.chunk);
            }

            Mode::SearchInput | Mode::SearchConfirm => {
                f.render_widget(r, self.chunk);

                self.search_widget.render(
                    f,
                    self.mode.is_search_input(),
                    self.item.highlight_status(),
                );
            }
        }
    }
}
