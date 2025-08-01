use std::rc::Rc;

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, HighlightSpacing, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    define_callback,
    ui::{
        event::{Callback, EventResult},
        key_event_to_code,
        util::{MousePosition, RectContainsPoint as _},
        Window,
    },
};

use super::{CheckListItem, Item, RenderTrait, SelectedItem, WidgetBase, WidgetTrait};

define_callback!(pub OnChangeCallback, Fn(&mut Window, &CheckListItem) -> EventResult);
define_callback!(pub RenderBlockInjection, Fn(&CheckList, bool) -> Block<'static>);

#[derive(Debug, Clone)]
pub struct CheckListTheme {
    // カーソル（選択中）が当たっているアイテムのスタイル
    pub selected: Style,

    // カーソル（選択中）のシンボル
    pub selected_symbol: String,

    // 必須項目のスタイル
    pub required: Style,

    // 必須項目のシンボル
    pub required_symbol: String,

    // チェック済みのアイテムのシンボル
    pub checked_symbol: String,

    // チェックされていないアイテムのシンボル
    pub unchecked_symbol: String,
}

impl Default for CheckListTheme {
    fn default() -> Self {
        Self {
            selected: Style::new().add_modifier(Modifier::REVERSED),
            required: Style::new().fg(Color::DarkGray),
            required_symbol: "(required)".to_string(),
            selected_symbol: ">".to_string(),
            checked_symbol: "[x]".to_string(),
            unchecked_symbol: "[ ]".to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct CheckListBuilder {
    id: String,
    widget_base: WidgetBase,
    theme: CheckListTheme,
    items: Vec<CheckListItem>,
    state: ListState,
    on_change: Option<OnChangeCallback>,
    block_injection: Option<RenderBlockInjection>,
}

#[allow(dead_code)]
impl CheckListBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_base(mut self, widget_base: WidgetBase) -> Self {
        self.widget_base = widget_base;
        self
    }

    pub fn theme(mut self, theme: CheckListTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn items<I>(mut self, items: impl IntoIterator<Item = I>) -> Self
    where
        I: Into<CheckListItem>,
    {
        self.items = items.into_iter().map(Into::into).collect();

        if !self.items.is_empty() {
            self.state.select(Some(0));
        }

        self
    }

    pub fn on_change<F>(mut self, on_change: F) -> Self
    where
        F: Into<OnChangeCallback>,
    {
        self.on_change = Some(on_change.into());
        self
    }

    pub fn block_injection<F>(mut self, block_injection: F) -> Self
    where
        F: Into<RenderBlockInjection>,
    {
        self.block_injection = Some(block_injection.into());
        self
    }

    pub fn build(self) -> CheckList {
        CheckList {
            id: self.id,
            widget_base: self.widget_base,
            theme: self.theme,
            items: self.items,
            state: self.state,
            chunk: Rect::default(),
            main_chunk: Rect::default(),
            footer_chunk: Rect::default(),
            on_change: self.on_change,
            block_injection: self.block_injection,
        }
    }
}

#[derive(Debug, Default)]
pub struct CheckList {
    id: String,
    widget_base: WidgetBase,
    theme: CheckListTheme,
    items: Vec<CheckListItem>,
    state: ListState,
    chunk: Rect,
    main_chunk: Rect,
    footer_chunk: Rect,
    on_change: Option<OnChangeCallback>,
    block_injection: Option<RenderBlockInjection>,
}

impl CheckList {
    pub fn builder() -> CheckListBuilder {
        CheckListBuilder::default()
    }

    pub fn items(&self) -> &[CheckListItem] {
        &self.items
    }

    fn layout() -> Layout {
        Layout::vertical([
            Constraint::Fill(1),   // Main content
            Constraint::Length(1), // Footer
        ])
    }

    fn toggle_selected(&mut self) -> bool {
        let Some(selected) = self.state.selected() else {
            return false;
        };

        if selected >= self.items.len() {
            return false;
        }

        let item = &mut self.items[selected];

        if item.required {
            // If the item is required, we do not toggle it.
            return false;
        }

        self.items[selected].checked = !self.items[selected].checked;

        true
    }

    fn move_selection(&mut self, offset: isize) -> bool {
        let Some(selected) = self.state.selected() else {
            return false;
        };

        let new_index = selected.saturating_add_signed(offset);

        if self.items.len() <= new_index {
            return false;
        }

        self.items.swap(selected, new_index);
        self.state.select(Some(new_index));

        true
    }

    fn on_change(&self) -> Option<Callback> {
        self.on_change.clone().and_then(|cb| {
            self.selected_item()
                .map(|v| Callback::new(move |w| cb(w, &v)))
        })
    }

    fn selected_item(&self) -> Option<Rc<CheckListItem>> {
        self.state
            .selected()
            .and_then(|index| self.items().get(index).map(|item| Rc::new(item.clone())))
    }
}

impl WidgetTrait for CheckList {
    fn id(&self) -> &str {
        &self.id
    }

    fn widget_base(&self) -> &WidgetBase {
        &self.widget_base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget_base
    }

    fn can_activate(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<SelectedItem> {
        self.state
            .selected()
            .and_then(|index| self.items().get(index).map(|item| item.clone().into()))
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, index: usize) {
        let selected = index.min(self.items.len().saturating_sub(1));

        self.state.select(Some(selected));
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

        self.state.select(Some(0));
    }

    fn select_last(&mut self) {
        if self.items.is_empty() {
            return;
        }

        self.state.select(Some(self.items.len().saturating_sub(1)))
    }

    fn append_widget_item(&mut self, _: Item) {
        todo!()
    }

    fn update_widget_item(&mut self, _: Item) {
        todo!()
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        if self.items.is_empty() {
            return EventResult::Nop;
        }

        let y = ev.row.saturating_sub(self.main_chunk.top()) as usize;

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if !self.main_chunk.contains_point(ev.position()) {
                    return EventResult::Nop;
                }

                if self.items.len() <= y + self.state.offset() {
                    return EventResult::Nop;
                }

                self.state.select(Some(y + self.state.offset()));

                let is_toggled = self.toggle_selected();

                if !is_toggled {
                    return EventResult::Nop;
                }

                if let Some(on_change) = self.on_change() {
                    return EventResult::Callback(on_change);
                }
            }
            MouseEventKind::Down(_mouse_button) => {}
            MouseEventKind::Up(_mouse_button) => {}
            MouseEventKind::Drag(_mouse_button) => {}
            MouseEventKind::Moved => {}
            MouseEventKind::ScrollDown => {}
            MouseEventKind::ScrollUp => {}
            MouseEventKind::ScrollLeft => {}
            MouseEventKind::ScrollRight => {}
        }

        EventResult::Nop
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match key_event_to_code(ev) {
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

            KeyCode::Char(' ') | KeyCode::Enter => {
                let is_toggled = self.toggle_selected();

                if !is_toggled {
                    return EventResult::Nop;
                }

                if let Some(on_change) = self.on_change() {
                    return EventResult::Callback(on_change);
                }
            }

            KeyCode::Char('J') => {
                let is_moved = self.move_selection(1);

                if !is_moved {
                    return EventResult::Nop;
                }

                if let Some(on_change) = self.on_change() {
                    return EventResult::Callback(on_change);
                }
            }

            KeyCode::Char('K') => {
                let is_moved = self.move_selection(-1);

                if !is_moved {
                    return EventResult::Nop;
                }

                if let Some(on_change) = self.on_change() {
                    return EventResult::Callback(on_change);
                }
            }

            _ => {
                return EventResult::Ignore;
            }
        }
        EventResult::Nop
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let inner_chunk = self.widget_base.block().inner(chunk);
        let [main_chunk, footer_chunk] = Self::layout().areas(inner_chunk);

        self.main_chunk = main_chunk;
        self.footer_chunk = footer_chunk;
    }

    fn clear(&mut self) {
        self.items.clear();
        self.state = ListState::default();
    }
}

impl CheckList {
    fn render_footer(&self, f: &mut Frame) {
        let widget = Paragraph::new(Line::from(vec![
            Span::raw(" Press ["),
            Span::styled("Space", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("/"),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("] to toggle, ["),
            Span::styled("J", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("/"),
            Span::styled("K", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("] to move."),
        ]));
        f.render_widget(widget, self.footer_chunk);
    }
}

impl RenderTrait for CheckList {
    fn render(&mut self, f: &mut Frame, is_active: bool, is_mouse_over: bool) {
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, is_active)
        } else {
            self.widget_base
                .render_block(self.can_activate() && is_active, is_mouse_over)
        };

        let items = create_list_items(&self.items, &self.theme);

        let widget = ratatui::widgets::List::new(items)
            .highlight_style(self.theme.selected)
            .highlight_symbol(self.theme.selected_symbol.as_str())
            .highlight_spacing(HighlightSpacing::Always);

        f.render_widget(block, self.chunk);

        f.render_stateful_widget(widget, self.main_chunk, &mut self.state);

        self.render_footer(f);
    }
}

fn create_list_items<'a, 'b>(
    items: &'a [CheckListItem],
    theme: &'a CheckListTheme,
) -> Vec<ListItem<'b>> {
    items
        .iter()
        .map(|item| {
            if item.required {
                ListItem::new(Line::styled(
                    format!(
                        " {} {} {}",
                        theme.checked_symbol, item.label, theme.required_symbol,
                    ),
                    theme.required,
                ))
            } else {
                let symbol = if item.checked {
                    &theme.checked_symbol
                } else {
                    &theme.unchecked_symbol
                };
                ListItem::new(Line::from(format!(" {} {}", symbol, item.label)))
            }
        })
        .collect()
}
