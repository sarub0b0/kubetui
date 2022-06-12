mod item;
mod render;
mod styled_graphemes;
mod wrap;

use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use derivative::*;
use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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

#[derive(Debug)]
struct SearchForm<'a> {
    input_widget: InputForm<'a>,
    chunks: Vec<Rect>,
}

impl Default for SearchForm<'_> {
    fn default() -> Self {
        Self {
            input_widget: InputForm::new(WidgetConfig::builder().block(Block::default()).build()),
            chunks: Default::default(),
        }
    }
}

impl<'a> SearchForm<'a> {
    fn update_chunk(&mut self, chunk: Rect) {
        let Rect {
            x,
            y: _,
            width,
            height,
        } = chunk;

        self.chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(width.saturating_sub(18 + 1)),
                Constraint::Length(15),
            ])
            .split(Rect::new(x, height.saturating_sub(1), width, 1));

        self.input_widget.update_chunk(self.chunks[1]);
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.input_widget.on_key_event(ev)
    }

    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool, status: (usize, usize))
    where
        B: Backend,
    {
        f.render_widget(Paragraph::new("Search: "), self.chunks[0]);

        self.input_widget.render(f, selected);

        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)).alignment(Alignment::Right),
            self.chunks[2],
        );
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
    pub fn search(&mut self, word: &str) {
        self.mode.search_confirm();

        // test
        word.chars().for_each(|c| {
            self.search_widget.input_widget.insert_char(c);
        });

        self.item.highlight(word);

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

    fn on_mouse_event(&mut self, _: MouseEvent) -> EventResult {
        todo!()
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

                KeyCode::Char('/') => {
                    self.mode.search_input();
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
                _ => {
                    return self.search_widget.on_key_event(ev);
                }
            },
        }

        EventResult::Nop
    }

    fn update_chunk(&mut self, chunk: Rect) {
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

        if self.mode.is_search_input() {
            let chunk = {
                let Rect {
                    x,
                    y,
                    width,
                    height,
                } = self.chunk;

                Rect::new(x, y, width, height.saturating_sub(1))
            };

            f.render_widget(r, chunk);

            self.search_widget.render(f, self.item.highlight_status());
        } else {
            f.render_widget(r, self.chunk);
        }
    }
}
