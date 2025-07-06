// mod filter_form;
mod filter;
mod item;

use std::rc::Rc;

use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{
        Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Table as TuiTable, TableState,
    },
};

use crate::{
    define_callback, logger,
    message::UserEvent,
    ui::{
        Window,
        event::{Callback, EventResult},
        key_event_to_code,
        util::{MousePosition, RectContainsPoint},
    },
};

use super::{
    Item, RenderTrait, SelectedItem, TableItem, WidgetTrait, base::WidgetBase, styled_graphemes,
};

pub use filter::{FilterForm, FilterFormTheme};

use item::InnerItem;

const COLUMN_SPACING: u16 = 3;
const HIGHLIGHT_SYMBOL: &str = " ";
const ROW_START_INDEX: usize = 2;

define_callback!(pub OnSelectCallback, Fn(&mut Window, &TableItem) -> EventResult);
define_callback!(pub RenderBlockInjection, Fn(&Table) -> WidgetBase);
define_callback!(pub RenderHighlightInjection, Fn(Option<&TableItem>) -> Style);

#[derive(Debug)]
pub struct TableTheme {
    header_style: Style,
}

impl Default for TableTheme {
    fn default() -> Self {
        Self {
            header_style: Style::default().fg(Color::DarkGray),
        }
    }
}

impl TableTheme {
    pub fn header_style(mut self, style: impl Into<Style>) -> Self {
        self.header_style = style.into();
        self
    }
}

#[derive(Debug, Default)]
pub struct TableBuilder {
    id: String,
    widget_base: WidgetBase,
    filter_form: FilterForm,
    theme: TableTheme,
    header: Vec<String>,
    items: Vec<TableItem>,
    state: TableState,
    filtered_key: String,
    on_select: Option<OnSelectCallback>,
    actions: Vec<(UserEvent, Callback)>,
    block_injection: Option<RenderBlockInjection>,
    highlight_injection: Option<RenderHighlightInjection>,
}

#[allow(dead_code)]
impl TableBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_base(mut self, widget_base: WidgetBase) -> Self {
        self.widget_base = widget_base;
        self
    }

    pub fn filter_form(mut self, filter_form: FilterForm) -> Self {
        self.filter_form = filter_form;
        self
    }

    pub fn theme(mut self, theme: TableTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn items(mut self, items: impl Into<Vec<TableItem>>) -> Self {
        self.items = items.into();
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
        self
    }

    pub fn header(mut self, header: impl Into<Vec<String>>) -> Self {
        self.header = header.into();
        self
    }

    pub fn filtered_key(mut self, key: impl Into<String>) -> Self {
        self.filtered_key = key.into();
        self
    }

    pub fn on_select<F>(mut self, cb: F) -> Self
    where
        F: Into<OnSelectCallback>,
    {
        self.on_select = Some(cb.into());
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

    pub fn highlight_injection<F>(mut self, highlight_injection: F) -> Self
    where
        F: Into<RenderHighlightInjection>,
    {
        self.highlight_injection = Some(highlight_injection.into());
        self
    }

    pub fn build(self) -> Table<'static> {
        let mut table = Table {
            id: self.id,
            widget_base: self.widget_base,
            theme: self.theme,
            on_select: self.on_select,
            actions: self.actions,
            state: self.state,
            block_injection: self.block_injection,
            highlight_injection: self.highlight_injection,
            filtered_key: self.filtered_key.clone(),
            filter_form: self.filter_form,
            ..Default::default()
        };

        table.items = InnerItem::builder()
            .header(self.header)
            .items(self.items)
            .filtered_key(self.filtered_key)
            .build();

        table.update_row_bounds();

        table
    }
}

#[derive(Debug)]
enum Mode {
    /// 通常（検索フォーム非表示）
    Normal,
    /// フィルターワード入力中（検索フォーム表示）
    FilterInput,
    /// フィルターワード確定後（検索フォーム表示）
    FilterConfirm,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Normal
    }
}

impl Mode {
    fn normal(&mut self) {
        *self = Self::Normal;
    }

    fn filter_input(&mut self) {
        *self = Self::FilterInput;
    }

    fn filter_confirm(&mut self) {
        *self = Self::FilterConfirm;
    }

    #[allow(dead_code)]
    fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }

    fn is_filter_input(&self) -> bool {
        matches!(self, Self::FilterInput)
    }

    fn is_filter_confirm(&self) -> bool {
        matches!(self, Self::FilterConfirm)
    }
}

#[derive(Debug, Default)]
pub struct Table<'a> {
    id: String,
    widget_base: WidgetBase,
    theme: TableTheme,
    items: InnerItem<'a>,
    state: TableState,
    chunk: Rect,
    row_bounds: Vec<(usize, usize)>,
    filter_form: FilterForm,
    filtered_key: String,
    mode: Mode,
    on_select: Option<OnSelectCallback>,
    actions: Vec<(UserEvent, Callback)>,
    block_injection: Option<RenderBlockInjection>,
    highlight_injection: Option<RenderHighlightInjection>,
}

impl Table<'_> {
    pub fn builder() -> TableBuilder {
        TableBuilder::default()
    }

    pub fn items(&self) -> &[TableItem] {
        self.items.items()
    }

    pub fn state(&self) -> &TableState {
        &self.state
    }

    pub fn equal_header(&self, header: &[String]) -> bool {
        self.items.header().original() == header
    }

    fn max_width(&self) -> usize {
        self.inner_chunk().width.saturating_sub(2) as usize
    }

    pub fn update_header_and_rows(&mut self, header: &[String], rows: &[TableItem]) {
        let old_len = self.items.len();

        self.items = InnerItem::builder()
            .header(header)
            .items(rows)
            .filtered_key(self.filtered_key.clone())
            .max_width(self.max_width())
            .build();

        self.items.update_filter(self.filter_form.content());

        self.adjust_selected(old_len, self.items.len());

        self.update_row_bounds();
    }

    fn update_row_bounds(&mut self) {
        let item_margin = self.items.item_margin() as usize;
        self.row_bounds = self
            .items
            .rendered_items()
            .iter()
            .scan(0, |sum, row| {
                let b = (*sum, *sum + row.height.saturating_sub(1));
                *sum += row.height + item_margin;
                Some(b)
            })
            .collect();
    }

    fn showable_height(&self) -> usize {
        self.inner_chunk().height.saturating_sub(2) as usize
    }

    fn max_offset(&self) -> usize {
        self.items
            .items()
            .len()
            .saturating_sub(self.showable_height())
    }

    // リストの下に空行があるとき、空行がなくなるようoffsetを調整する
    fn adjust_offset(&mut self) {
        let shown_item_len = self.items.len().saturating_sub(self.state.offset());
        let showable_height = self.showable_height();
        if shown_item_len < showable_height {
            *self.state.offset_mut() = self.max_offset();
        }
    }

    fn chunk(&self) -> Rect {
        let Rect {
            x,
            y,
            width,
            height,
        } = self.chunk;

        match self.mode {
            Mode::Normal => self.chunk,

            Mode::FilterInput | Mode::FilterConfirm => {
                let filter_height = self.filter_form.form_height();

                Rect::new(
                    x,
                    y + filter_height,
                    width,
                    height.saturating_sub(filter_height),
                )
            }
        }
    }

    fn inner_chunk(&self) -> Rect {
        self.widget_base.block().inner(self.chunk())
    }

    fn filter_items(&mut self) {
        let old_len = self.items.len();

        self.items.update_filter(self.filter_form.content());

        self.adjust_selected(old_len, self.items.len());

        self.update_row_bounds();
    }

    fn adjust_selected(&mut self, prev: usize, next: usize) {
        match next {
            // アイテムがなくなったとき
            0 => self.state = Default::default(),

            // アイテムが減った場合
            next if next < prev => {
                // 選択中アイテムインデックスよりもアイテムが減少したとき一番下のアイテムを選択する

                if let Some(selected) = self.state.selected() {
                    if next <= selected {
                        self.select_last();
                    }
                }

                self.adjust_offset();
            }

            // アイテムが増えた場合
            _ => {
                if self.state.selected().is_none() {
                    self.state.select(Some(0));
                }
            }
        }
    }

    fn filter_cancel(&mut self) {
        self.mode.normal();

        self.filter_form.clear();

        self.filter_items();
    }
}

impl WidgetTrait for Table<'_> {
    fn id(&self) -> &str {
        &self.id
    }

    fn can_activate(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<SelectedItem> {
        self.state()
            .selected()
            .and_then(|index| self.items().get(index).map(|item| item.clone().into()))
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        todo!()
    }

    fn select_next(&mut self, index: usize) {
        if self.items.is_empty() {
            return;
        }

        let current = self.state.selected().unwrap_or(0);

        let selected = (current + index).min(self.items.len().saturating_sub(1));

        self.state.select(Some(selected));
    }

    fn select_prev(&mut self, index: usize) {
        if self.items.is_empty() {
            return;
        }

        let current = self.state.selected().unwrap_or(0);

        let selected = current
            .saturating_sub(index)
            .min(self.items.len().saturating_sub(1));

        self.state.select(Some(selected));
    }

    fn select_first(&mut self) {
        if self.items.is_empty() {
            return;
        }

        self.state.select(Some(0))
    }

    fn select_last(&mut self) {
        if self.items.is_empty() {
            return;
        }

        self.state.select(Some(self.items.len().saturating_sub(1)))
    }

    fn append_widget_item(&mut self, _: Item) {
        unimplemented!()
    }

    /// Widgetのアイテム更新と更新時にスクロールの制御を行う
    ///
    /// # Arguments
    /// * `items` - 更新するアイテム
    ///
    fn update_widget_item(&mut self, items: Item) {
        let old_len = self.items.len();

        self.items.update_items(items.table());

        self.adjust_selected(old_len, self.items.len());

        self.update_row_bounds();
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        if self.items.is_empty() {
            return EventResult::Nop;
        }

        let inner_chunk = self.inner_chunk();

        let (_, row) = (
            ev.column.saturating_sub(inner_chunk.left()) as usize,
            ev.row.saturating_sub(inner_chunk.top()) as usize,
        );

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if !inner_chunk.contains_point(ev.position()) {
                    return EventResult::Nop;
                }

                let offset_index = self.state.offset();
                let offset_bound = self.row_bounds[offset_index];
                let offset_row = offset_bound.0;

                let header_margin = if self.items.header().is_empty() {
                    0
                } else {
                    ROW_START_INDEX
                };

                if let Some((index, _)) =
                    self.row_bounds[offset_index..]
                        .iter()
                        .enumerate()
                        .find(|(_, b)| {
                            let b = (
                                b.0.saturating_sub(offset_row) + header_margin,
                                b.1.saturating_sub(offset_row) + header_margin,
                            );

                            b.0 <= row && row <= b.1
                        })
                {
                    self.state.select(Some(index + offset_index));

                    if let Some(cb) = self.on_select_callback() {
                        return EventResult::Callback(cb);
                    }

                    return EventResult::Ignore;
                }
            }

            MouseEventKind::ScrollDown => {
                self.select_next(1);
                return EventResult::Nop;
            }
            MouseEventKind::ScrollUp => {
                self.select_prev(1);
                return EventResult::Nop;
            }
            _ => {}
        }

        EventResult::Ignore
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match self.mode {
            Mode::Normal | Mode::FilterConfirm => match key_event_to_code(ev) {
                KeyCode::Char('j') | KeyCode::Down | KeyCode::PageDown => {
                    self.select_next(1);
                }

                KeyCode::Char('k') | KeyCode::Up | KeyCode::PageUp => {
                    self.select_prev(1);
                }

                KeyCode::Char('G') | KeyCode::End => {
                    self.select_last();
                }

                KeyCode::Char('g') | KeyCode::Home => {
                    self.select_first();
                }

                KeyCode::Char('/') => {
                    self.mode.filter_input();
                }

                KeyCode::Char('q') | KeyCode::Esc if self.mode.is_filter_confirm() => {
                    self.filter_cancel();
                }

                KeyCode::Enter => {
                    if let Some(cb) = self.on_select_callback() {
                        return EventResult::Callback(cb);
                    }

                    return EventResult::Ignore;
                }

                _ => {
                    if let Some(cb) = self.match_action(UserEvent::Key(ev)) {
                        return EventResult::Callback(cb.clone());
                    }

                    return EventResult::Ignore;
                }
            },

            Mode::FilterInput => match key_event_to_code(ev) {
                KeyCode::Enter => {
                    self.mode.filter_confirm();
                }

                KeyCode::Esc => {
                    self.filter_cancel();
                }

                _ => {
                    let ev = self.filter_form.on_key_event(ev);

                    self.filter_items();

                    return ev;
                }
            },
        }

        EventResult::Nop
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        self.items.update_max_width(self.max_width());

        self.adjust_offset();

        self.update_row_bounds();

        let filter_height = self.filter_form.form_height();

        self.filter_form
            .update_chunk(Rect::new(chunk.x, chunk.y, chunk.width, filter_height));
    }

    fn clear(&mut self) {
        self.state = TableState::default();

        self.items = InnerItem::builder()
            .max_width(self.max_width())
            .filtered_key(self.filtered_key.clone())
            .build();

        self.row_bounds = Vec::default();

        *(self.widget_base.append_title_mut()) = None;
    }

    fn widget_base(&self) -> &WidgetBase {
        &self.widget_base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget_base
    }
}

impl Table<'_> {
    fn on_select_callback(&self) -> Option<Callback> {
        self.on_select.clone().and_then(|cb| {
            self.selected_item()
                .map(|v| Callback::new(move |w| cb(w, &v)))
        })
    }

    fn selected_item(&self) -> Option<Rc<TableItem>> {
        self.state
            .selected()
            .and_then(|index| self.items().get(index).map(|item| Rc::new(item.clone())))
    }

    fn match_action(&self, ev: UserEvent) -> Option<&Callback> {
        self.actions
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb) } else { None })
    }
}

impl Table<'_> {
    fn render_highlight_style(&self) -> Style {
        if let Some(highlight_injection) = &self.highlight_injection {
            highlight_injection(self.selected_item().as_deref())
        } else if let Some(item) = self.selected_item() {
            let mut style = Style::default().add_modifier(Modifier::REVERSED);

            if let Some(item) = item.item.first() {
                let sg = styled_graphemes::styled_graphemes(item);

                if let Some(first) = sg.first() {
                    if let Some(fg) = first.style().fg {
                        style = Style::default().fg(fg).add_modifier(Modifier::REVERSED);
                    }
                }
            }
            style
        } else {
            Style::default().add_modifier(Modifier::REVERSED)
        }
    }
}

impl RenderTrait for Table<'_> {
    fn render(&mut self, f: &mut Frame<'_>, is_active: bool, is_mouse_over: bool) {
        let widget_base = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self)
        } else {
            self.widget_base.clone()
        };

        let block = widget_base.render_block(
            self.can_activate() && !self.mode.is_filter_input() && is_active,
            is_mouse_over,
        );

        let chunk = self.chunk();

        if self.items.is_empty() {
            let paragraph = Paragraph::new(" No data".dark_gray()).block(block);
            f.render_widget(paragraph, chunk);
        } else {
            let constraints = constraints(self.items.digits());

            let highlight_style = self.render_highlight_style();

            let mut widget = TuiTable::new(self.items.to_rendered_rows(), constraints)
                .block(block)
                .row_highlight_style(highlight_style)
                .highlight_symbol(HIGHLIGHT_SYMBOL)
                .column_spacing(COLUMN_SPACING);

            if !self.items.header().is_empty() {
                widget = widget.header(
                    self.items
                        .header()
                        .rendered()
                        .style(self.theme.header_style),
                );
            }

            f.render_stateful_widget(widget, chunk, &mut self.state);
        }

        match self.mode {
            Mode::Normal => {}
            Mode::FilterInput | Mode::FilterConfirm => {
                self.filter_form
                    .render(f, self.mode.is_filter_input() && is_active, false);
            }
        }

        logger!(debug, "{:?}", self.items);

        logger!(
            debug,
            "selected {:?}, offset {} ",
            self.state.selected(),
            self.state.offset()
        );

        let mut scrollbar_state = ScrollbarState::default()
            .position(self.state.offset())
            .content_length(self.max_offset())
            .viewport_content_length(2);

        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            chunk,
            &mut scrollbar_state,
        )
    }
}

fn constraints(digits: &[usize]) -> Vec<Constraint> {
    digits
        .iter()
        .map(|d| Constraint::Length(*d as u16))
        .collect()
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    mod カラムの幅 {
        use super::*;

        #[test]
        fn 表示する文字列幅でカラム幅を計算する() {
            let item = InnerItem::builder()
                .header(["\x1b[0mA".to_string(), "B".to_string()])
                .items([TableItem::new(
                    ["abc".to_string(), "\x1b[31mabc\x1b[0m".to_string()],
                    None,
                )])
                .max_width(usize::MAX)
                .build();

            assert_eq!(item.digits(), vec![3, 3])
        }
    }

    mod 選択アイテムの切り替え {
        use super::*;

        #[test]
        fn 初期値は1つ目のアイテムを選択() {
            let table = Table::builder()
                .items([
                    TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                    TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                    TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                    TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                    TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                ])
                .build();

            assert_eq!(table.state.selected(), Some(0))
        }

        mod select_next {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn アイテムが空のとき未選択() {
                let mut table = Table::builder().build();

                table.select_next(1);

                assert_eq!(table.state.selected(), None)
            }

            #[test]
            fn 指定した数分増加したインデックスのアイテムを選択() {
                let mut table = Table::builder()
                    .items([
                        TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                        TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                        TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                        TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                        TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                    ])
                    .build();

                table.select_next(1);

                assert_eq!(table.state.selected(), Some(1));

                table.select_next(2);

                assert_eq!(table.state.selected(), Some(3));
            }

            #[test]
            fn アイテム数を超えて選択できない() {
                let mut table = Table::builder()
                    .items([
                        TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                        TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                        TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                        TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                        TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                    ])
                    .build();

                table.select_next(100);

                assert_eq!(table.state.selected(), Some(4));
            }
        }

        mod select_prev {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn アイテムが空のとき未選択() {
                let mut table = Table::builder().build();

                table.select_prev(1);

                assert_eq!(table.state.selected(), None)
            }

            #[test]
            fn 指定した数分減算したインデックスのアイテムを選択() {
                let mut table = Table::builder()
                    .items([
                        TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                        TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                        TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                        TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                        TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                    ])
                    .build();

                table.state.select(Some(4));

                table.select_prev(1);

                assert_eq!(table.state.selected(), Some(3));

                table.select_prev(2);

                assert_eq!(table.state.selected(), Some(1));
            }

            #[test]
            fn インデックスは0より小さい値を選択できない() {
                let mut table = Table::builder()
                    .items([
                        TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                        TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                        TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                        TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                        TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                    ])
                    .build();

                table.select_prev(100);

                assert_eq!(table.state.selected(), Some(0));
            }
        }

        mod select_first {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn アイテムが空のとき未選択() {
                let mut table = Table::builder().build();

                table.select_first();

                assert_eq!(table.state.selected(), None)
            }

            #[test]
            fn インデックス0のアイテムを選択() {
                let mut table = Table::builder()
                    .items([
                        TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                        TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                        TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                        TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                        TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                    ])
                    .build();

                table.state.select(Some(4));

                table.select_first();

                assert_eq!(table.state.selected(), Some(0))
            }
        }

        mod select_last {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn アイテムが空のとき未選択() {
                let mut table = Table::builder().build();

                table.select_last();

                assert_eq!(table.state.selected(), None)
            }

            #[test]
            fn 一番下のアイテムを選択() {
                let mut table = Table::builder()
                    .items([
                        TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                        TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                        TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                        TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                        TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                    ])
                    .build();

                table.select_last();

                assert_eq!(table.state.selected(), Some(4))
            }
        }
    }

    struct TestData {
        terminal: Terminal<TestBackend>,
        table: Table<'static>,
    }

    mod アイテム変更時のアイテム選択位置とスクロール調整 {
        use super::*;

        fn setup() -> TestData {
            let backend = TestBackend::new(22, 7);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut table = Table::builder()
                .header(["A".to_string(), "B".to_string()])
                .items([
                    TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                    TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                    TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                    TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                    TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                    TableItem::new(vec!["Item-5".to_string(), "Item-5".to_string()], None),
                    TableItem::new(vec!["Item-6".to_string(), "Item-6".to_string()], None),
                    TableItem::new(vec!["Item-7".to_string(), "Item-7".to_string()], None),
                    TableItem::new(vec!["Item-8".to_string(), "Item-8".to_string()], None),
                    TableItem::new(vec!["Item-9".to_string(), "Item-9".to_string()], None),
                ])
                .build();

            let chunk = Rect::new(0, 0, 22, 7);
            table.update_chunk(chunk);

            terminal
                .draw(|f| {
                    table.render(f, true, false);
                })
                .unwrap();

            TestData { terminal, table }
        }

        mod アイテム増加時 {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn アイテムが空の時に追加されると1つ目のアイテムを選択する() {
                let chunk = Rect::new(0, 0, 10, 5);
                let mut table = Table::default();
                table.update_chunk(chunk);

                assert_eq!((table.state.selected(), table.state.offset()), (None, 0));

                table.update_widget_item(Item::Table(vec![
                    TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                    TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                    TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                ]));

                assert_eq!((table.state.selected(), table.state.offset()), (Some(0), 0));
            }

            #[test]
            fn アイテムが空でないときに追加されても選択位置とオフセットは変化しない() {
                let TestData {
                    mut table,
                    mut terminal,
                } = setup();

                table.select_next(5);

                terminal
                    .draw(|f| {
                        table.render(f, true, false);
                    })
                    .unwrap();

                assert_eq!((table.state.selected(), table.state.offset()), (Some(5), 3));

                table.update_widget_item(Item::Table(vec![
                    TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                    TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                    TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                    TableItem::new(vec!["Item-3".to_string(), "Item-3".to_string()], None),
                    TableItem::new(vec!["Item-4".to_string(), "Item-4".to_string()], None),
                    TableItem::new(vec!["Item-5".to_string(), "Item-5".to_string()], None),
                    TableItem::new(vec!["Item-6".to_string(), "Item-6".to_string()], None),
                    TableItem::new(vec!["Item-7".to_string(), "Item-7".to_string()], None),
                    TableItem::new(vec!["Item-8".to_string(), "Item-8".to_string()], None),
                    TableItem::new(vec!["Item-9".to_string(), "Item-9".to_string()], None),
                    // 増加分
                    TableItem::new(vec!["Item-10".to_string(), "Item-10".to_string()], None),
                    TableItem::new(vec!["Item-11".to_string(), "Item-11".to_string()], None),
                    TableItem::new(vec!["Item-12".to_string(), "Item-12".to_string()], None),
                ]));

                terminal
                    .draw(|f| {
                        table.render(f, true, false);
                    })
                    .unwrap();

                assert_eq!((table.state.selected(), table.state.offset()), (Some(5), 3));
            }
        }

        mod アイテム減少時 {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn 選択中アイテムインデックスよりもアイテム数が減少したとき一番下のアイテムを選択する()
            {
                let TestData {
                    mut table,
                    mut terminal,
                } = setup();

                table.select_next(8);

                terminal
                    .draw(|f| {
                        table.render(f, false, false);
                    })
                    .unwrap();

                assert_eq!((table.state.selected(), table.state.offset()), (Some(8), 6));

                table.update_widget_item(Item::Table(vec![
                    TableItem::new(vec!["Item-0".to_string(), "Item-0".to_string()], None),
                    TableItem::new(vec!["Item-1".to_string(), "Item-1".to_string()], None),
                    TableItem::new(vec!["Item-2".to_string(), "Item-2".to_string()], None),
                ]));

                terminal
                    .draw(|f| {
                        table.render(f, false, false);
                    })
                    .unwrap();

                assert_eq!((table.state.selected(), table.state.offset()), (Some(2), 0));
            }
        }

        mod アイテム削除時 {

            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn デフォルトが設定される() {
                let TestData {
                    mut table,
                    terminal: _,
                } = setup();

                table.update_widget_item(Item::Table(Vec::new()));

                assert_eq!((table.state.selected(), table.state.offset()), (None, 0));
            }
        }
    }
}
