mod item;
mod render;
mod search_form;
mod wrap;

use std::{cell::RefCell, rc::Rc};

use derivative::Derivative;

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
    widgets::{Block, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::{
    clipboard::Clipboard,
    define_callback, logger,
    message::UserEvent,
    ui::{
        event::{Callback, EventResult},
        key_event_to_code,
        util::{MousePosition, RectContainsPoint},
    },
};

use super::{
    config::WidgetConfig, styled_graphemes::StyledGrapheme, Item, LiteralItem, RenderTrait,
    SelectedItem, WidgetTrait,
};

use self::{
    highlight_content::{HighlightArea, HighlightContent, Point},
    item::TextItem,
    render::{Render, Scroll},
    search_form::SearchForm,
};

define_callback!(pub RenderBlockInjection, Fn(&Text, bool, bool) -> Block<'static> );

mod highlight_content {

    #[derive(Default, Debug, Copy, Clone)]
    pub struct Point {
        pub x: usize,
        pub y: usize,
    }

    /// ハイライトの開始位置を終了位置を管理
    /// 絶対位置
    #[derive(Default, Debug, Copy, Clone)]
    pub struct HighlightArea {
        /// x, y
        pub start: Point,
        /// x, y
        pub end: Point,
    }

    impl HighlightArea {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn start(mut self, x: usize, y: usize) -> Self {
            self.start = Point { x, y };
            self
        }

        pub fn end(mut self, x: usize, y: usize) -> Self {
            self.end = Point { x, y };
            self
        }

        pub fn area(&self) -> Self {
            use std::mem::swap;

            let mut area = *self;

            if (area.end.y < area.start.y)
                || (area.start.y == area.end.y && area.end.x < area.start.x)
            {
                swap(&mut area.start, &mut area.end);
            }

            area
        }

        pub fn contains(&self, p: Point) -> bool {
            let area = self.area();

            let start = area.start;
            let end = area.end;

            if start.y <= p.y && p.y <= end.y {
                if start.y == p.y && p.x < start.x {
                    false
                } else {
                    !(end.y == p.y && end.x < p.x)
                }
            } else {
                false
            }
        }
    }

    #[derive(Default, Debug, Clone)]
    pub struct HighlightContent {
        /// 範囲選択されている座標
        pub area: HighlightArea,

        /// D&Dの間followをとめるためにTextItemに設定されているfollowを保存する
        pub follow: bool,
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
    actions: Vec<(UserEvent, Callback)>,
    #[derivative(Debug = "ignore")]
    clipboard: Option<Rc<RefCell<Clipboard>>>,
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

    pub fn action<F, E>(mut self, ev: E, cb: F) -> Self
    where
        E: Into<UserEvent>,
        F: Into<Callback>,
    {
        self.actions.push((ev.into(), cb.into()));
        self
    }

    pub fn block_injection<F>(mut self, block_injection: F) -> Self
    where
        F: Into<RenderBlockInjection>,
    {
        self.block_injection = Some(block_injection.into());
        self
    }

    pub fn clipboard(mut self, clipboard: Rc<RefCell<Clipboard>>) -> Self {
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
    highlight_content: Option<HighlightContent>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, Callback)>,
    #[derivative(Debug = "ignore")]
    clipboard: Option<Rc<RefCell<Clipboard>>>,
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
        let is_bottom = self.is_bottom();

        self.mode.search_input();

        if is_bottom {
            self.select_last()
        }

        let word = self.search_widget.word();

        if word.is_empty() {
            // 入力文字が空の時に1文字だけハイライトが残るのを防ぐため
            self.item.clear_highlight();
            return;
        }

        self.item.highlight(&word);

        if let Some(index) = self
            .item
            .select_nearest_highlight(self.search_nearest_highlight_target_index())
        {
            self.scroll.y = self.search_scroll(index);
        }
    }

    /// 次のマッチ箇所にスクロールする
    /// - マッチ箇所がchunk内の場合次のマッチ箇所に移動
    /// - マッチ箇所がchunk外の場合近いマッチ箇所に移動する
    pub fn search_next(&mut self) {
        if let Some(selected_line_number) = self.item.highlight_selected_line_number() {
            let index = if self.within_chunk(selected_line_number) {
                self.item.select_next_highlight()
            } else {
                self.item
                    .select_nearest_highlight(self.search_nearest_highlight_target_index())
            };

            if let Some(index) = index {
                self.scroll.y = self.search_scroll(index);
            }
        }
    }

    /// 前のマッチ箇所にスクロールする
    /// - マッチ箇所がchunk内の場合前のマッチ箇所に移動
    /// - マッチ箇所がchunk外の場合近いマッチ箇所に移動する
    pub fn search_prev(&mut self) {
        if let Some(selected_line_number) = self.item.highlight_selected_line_number() {
            let index = if self.within_chunk(selected_line_number) {
                self.item.select_prev_highlight()
            } else {
                self.item
                    .select_nearest_highlight(self.search_nearest_highlight_target_index())
            };

            if let Some(index) = index {
                self.scroll.y = self.search_scroll(index);
            }
        }
    }

    pub fn search_cancel(&mut self) {
        self.mode.normal();
        self.item.clear_highlight();

        if self.scroll_y_last_index() < self.scroll.y {
            self.select_last()
        }
    }

    /// 移動したいハイライトが中央になるスクロール位置を返す
    /// 画面内に収まる場合はスクロールしない
    fn search_scroll(&self, search_index: usize) -> usize {
        if self.within_chunk(search_index) {
            self.scroll.y
        } else {
            search_index
                .saturating_sub((self.inner_chunk().height as f32 * 0.5) as usize)
                .min(self.scroll_y_last_index())
        }
    }

    fn search_nearest_highlight_target_index(&self) -> usize {
        self.scroll.y + (self.inner_chunk().height as f32 * 0.5) as usize
    }

    fn within_chunk(&self, index: usize) -> bool {
        let min = self.scroll.y;
        let max = self.scroll.y + self.inner_chunk().height as usize;

        min <= index && index <= max
    }
}

impl Text {
    pub fn scroll_right(&mut self, i: usize) {
        if self.wrap {
            return;
        }

        self.scroll.x = self
            .scroll
            .x
            .saturating_add(i)
            .min(self.scroll_x_last_index());
    }

    pub fn scroll_left(&mut self, i: usize) {
        if self.wrap {
            return;
        }

        self.scroll.x = self.scroll.x.saturating_sub(i);
    }

    pub fn scroll_x_last_index(&self) -> usize {
        self.item
            .max_chars()
            .saturating_sub(self.inner_chunk().width as usize)
    }

    pub fn scroll_y_last_index(&self) -> usize {
        self.item
            .wrapped_lines()
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

    fn is_bottom(&self) -> bool {
        self.scroll_y_last_index() <= self.scroll.y
    }
}

impl Text {
    pub fn state(&self) -> (usize, usize) {
        (self.scroll.y, self.scroll_y_last_index())
    }

    fn match_action(&self, ev: UserEvent) -> Option<&Callback> {
        self.actions
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb) } else { None })
    }
}

impl Text {
    fn mouse_pos(&self, col: u16, row: u16) -> Point {
        let inner_chunk = self.inner_chunk();
        Point {
            x: col.saturating_sub(inner_chunk.left()) as usize,
            y: row.saturating_sub(inner_chunk.top()) as usize,
        }
    }
}

impl WidgetTrait for Text {
    fn id(&self) -> &str {
        &self.id
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }

    fn can_activate(&self) -> bool {
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
        let is_bottom = self.is_bottom();

        match item {
            Item::Single(i) => self.item.push(i),
            Item::Array(i) => self.item.extend(i),
            _ => {
                unreachable!()
            }
        }

        if self.follow && is_bottom {
            self.select_last()
        }
    }

    fn update_widget_item(&mut self, item: Item) {
        let is_bottom = self.is_bottom();

        let item = item.array();
        self.item.update(item);

        if self.follow && is_bottom {
            self.select_last()
        }

        if self.is_bottom() {
            self.select_last()
        }
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        if self.item.is_empty() {
            return EventResult::Nop;
        }

        let pos = self.mouse_pos(ev.column, ev.row);

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if !self.inner_chunk().contains_point(ev.position()) {
                    return EventResult::Nop;
                }
                // posに該当するWrappedLineとStyleGraphemeのインデックスを探す

                let (x, y) = (pos.x + self.scroll.x, pos.y + self.scroll.y);

                let area = HighlightArea::new().start(x, y).end(x, y);

                self.highlight_content = Some(HighlightContent {
                    area,
                    follow: self.follow,
                });

                self.follow = false;
            }

            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(highlight_content) = &mut self.highlight_content {
                    let (x, y) = (pos.x + self.scroll.x, pos.y + self.scroll.y);
                    highlight_content.area = highlight_content.area.end(x, y);
                }
            }

            // ハイライトの削除とクリップボードに保存
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(highlight_content) = &mut self.highlight_content {
                    let area = highlight_content.area.area();

                    let lines = &self.item.wrapped_lines();

                    let mut contents = String::new();

                    let start = area.start;
                    let end = Point {
                        x: area.end.x,
                        y: area.end.y.min(lines.len().saturating_sub(1)),
                    };

                    for i in start.y..=end.y {
                        let line = &lines[i];
                        let len = line.line().len().saturating_sub(1);

                        match i {
                            i if start.y == i && end.y == i => {
                                let start = start.x.min(len);
                                let end = end.x.min(len);

                                if let Some(content) = line.line().get(start..=end) {
                                    contents += &content
                                        .iter()
                                        .map(StyledGrapheme::symbol)
                                        .collect::<String>();
                                }
                            }
                            i if start.y == i => {
                                let start = start.x;

                                if len < start {
                                    continue;
                                }

                                if let Some(content) = line.line().get(start..) {
                                    contents += &content
                                        .iter()
                                        .map(StyledGrapheme::symbol)
                                        .collect::<String>();
                                }
                            }
                            i if end.y == i => {
                                let end = end.x.min(len);

                                if let Some(content) = line.line().get(..=end) {
                                    contents += &content
                                        .iter()
                                        .map(StyledGrapheme::symbol)
                                        .collect::<String>();
                                }
                            }
                            _ => {
                                contents += &line
                                    .line()
                                    .iter()
                                    .map(StyledGrapheme::symbol)
                                    .collect::<String>();
                            }
                        }

                        if i != end.y {
                            if let Some(next) = lines.get(i + 1) {
                                if line.index() != next.index() {
                                    contents.push('\n');
                                }
                            }
                        }
                    }

                    if let Some(clipboard) = &mut self.clipboard {
                        logger!(info, "Clipboard saved '{}'", contents);
                        if let Err(e) = clipboard.borrow_mut().set_contents(contents) {
                            logger!(error, "Clipboard Error '{}'", e);
                        }
                    }

                    self.follow = highlight_content.follow;
                }

                self.highlight_content = None;
            }
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

                Char('n') if !self.mode.is_normal() => {
                    self.search_next();
                }

                Char('N') if !self.mode.is_normal() => {
                    self.search_prev();
                }

                _ => {
                    if let Some(cb) = self.match_action(UserEvent::Key(ev)) {
                        return EventResult::Callback(cb.clone());
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
        let is_bottom = self.is_bottom();

        self.chunk = chunk;

        if self.wrap {
            self.item.rewrap(self.inner_chunk().width as usize);
        };

        self.search_widget.update_chunk(chunk);

        if self.follow && is_bottom {
            self.select_last()
        }

        if self.scroll_y_last_index() < self.scroll.y || is_bottom {
            self.select_last()
        }
    }

    fn clear(&mut self) {
        self.scroll = Default::default();

        let wrap_width = if self.wrap {
            Some(self.inner_chunk().width as usize)
        } else {
            None
        };

        self.item = TextItem::new(vec![], wrap_width);
        self.search_cancel();

        *(self.widget_config.append_title_mut()) = None;
    }
}

impl RenderTrait for Text {
    fn render(&mut self, f: &mut Frame<'_>, is_active: bool, is_mouse_over: bool) {
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, self.can_activate() && is_active, is_mouse_over)
        } else {
            self.widget_config
                .render_block(self.can_activate() && is_active, is_mouse_over)
        };

        let wrapped_lines = self.item.wrapped_lines();

        let mut builder = Render::builder()
            .block(block)
            .lines(wrapped_lines)
            .scroll(self.scroll);

        if let Some(highlight_content) = &self.highlight_content {
            builder = builder.highlight_area(Some(highlight_content.area));
        }

        let r = builder.build();

        match self.mode {
            Mode::Normal => {
                f.render_widget(r, self.chunk());
            }

            Mode::SearchInput | Mode::SearchConfirm => {
                f.render_widget(r, self.chunk());

                self.search_widget.render(
                    f,
                    self.mode.is_search_input() && is_active,
                    self.item.highlight_status(),
                );
            }
        }

        if !self.wrap {
            let mut scrollbar_state = ScrollbarState::default()
                .position(self.scroll.x)
                .content_length(self.scroll_x_last_index())
                .viewport_content_length(2);

            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                    .begin_symbol(None)
                    .end_symbol(None),
                self.chunk().inner(ratatui::prelude::Margin {
                    horizontal: 1,
                    vertical: 0,
                }),
                &mut scrollbar_state,
            )
        }

        let mut scrollbar_state = ScrollbarState::default()
            .position(self.scroll.y)
            .content_length(self.scroll_y_last_index())
            .viewport_content_length(2);

        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            self.chunk(),
            &mut scrollbar_state,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod スクロール {
        use super::*;

        mod アイテム更新 {
            use super::*;

            mod アイテム減少 {
                use super::*;

                #[test]
                fn 最大スクロール位置がアイテム数より大きいときスクロール位置を調整() {
                    // --------
                    // |0
                    // |1
                    // |2
                    // |3
                    // |4
                    // --------
                    let mut text = Text::builder()
                        .items([
                            "0".to_string(),
                            "1".to_string(),
                            "2".to_string(),
                            "3".to_string(),
                            "4".to_string(),
                            "5".to_string(),
                            "6".to_string(),
                            "7".to_string(),
                            "8".to_string(),
                            "9".to_string(),
                        ])
                        .build();

                    text.update_chunk(Rect::new(0, 0, 10, 7));

                    text.select_last();

                    // --------
                    // |5
                    // |6
                    // |7
                    // |8
                    // |9
                    // --------
                    assert_eq!(text.scroll.y, 5);

                    text.update_widget_item(Item::Array(vec![
                        LiteralItem::new("0", None),
                        LiteralItem::new("1", None),
                        LiteralItem::new("2", None),
                        LiteralItem::new("3", None),
                        LiteralItem::new("4", None),
                    ]));

                    // --------
                    // |0
                    // |1
                    // |2
                    // |3
                    // |4
                    // --------
                    assert_eq!(text.scroll.y, 0);
                }
            }
        }

        #[test]
        fn scroll_right() {
            let mut text = Text::builder()
                .items([
                    "0".to_string(),
                    "01234".to_string(),
                    "0123456789".to_string(),
                ])
                .build();

            text.update_chunk(Rect::new(0, 0, 5, 10));

            text.scroll_right(10);

            assert_eq!(text.scroll.x, 7);
        }
    }
}
