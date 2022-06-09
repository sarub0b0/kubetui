mod item;
mod render;
mod styled_graphemes;
mod wrap;

use std::rc::Rc;

use crossterm::event::{KeyEvent, MouseEvent};
use derivative::*;
use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::StyledGrapheme,
    widgets::{Block, Widget},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::tui_wrapper::event::EventResult;

use self::{
    item::TextItem,
    render::{Render, Scroll},
};

use super::{config::WidgetConfig, Item, LiteralItem, RenderTrait, SelectedItem, WidgetTrait};

type RenderBlockInjection = Rc<dyn Fn(&Text, bool) -> Block<'static>>;

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

    pub fn build(self) -> Text<'static> {
        let ret = Text {
            id: self.id,
            widget_config: self.widget_config,
            item: TextItem::new(self.item, None),
            wrap: self.wrap,
            follow: self.follow,
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
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
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

    fn on_key_event(&mut self, _: KeyEvent) -> EventResult {
        todo!()
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
        self.inner_chunk = self.widget_config.block().inner(chunk);

        if self.wrap {
            self.item.rewrap(self.inner_chunk.width as usize);
        };
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

        f.render_widget(r, self.chunk);
    }
}
