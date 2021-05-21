use chrono::Local;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use tui_wrapper::{widget::*, *};

use super::view_id;

pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    selected_tab_index: usize,
    layout: Layout,
    chunk: Rect,
}

pub mod window_layout_index {
    pub const TAB: usize = 0;
    pub const CONTEXT: usize = 2;
    pub const CONTENTS: usize = 3;
    pub const STATUSBAR: usize = 4;
}

// Window
impl<'a> Window<'a> {
    pub fn new(tabs: Vec<Tab<'a>>) -> Self {
        Self {
            tabs,
            selected_tab_index: 0,
            layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(1),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                ),
            chunk: Default::default(),
        }
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let chunks = self.layout.split(chunk);

        self.tabs.iter_mut().for_each(|tab| {
            tab.update_chunk(chunks[window_layout_index::CONTENTS]);
        });
    }

    pub fn chunks(&self) -> Vec<Rect> {
        self.layout.split(self.chunk)
    }

    pub fn widget(&self) -> Tabs {
        let titles: Vec<Spans> = self
            .tabs
            .iter()
            .map(|t| Spans::from(Self::tab_title_format(t.title())))
            .collect();

        Tabs::new(titles)
            .block(Self::tab_block())
            .select(self.selected_tab_index)
            .highlight_style(
                Style::default()
                    .bg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )
    }

    fn tab_title_format(title: &str) -> String {
        format!(" {} ", title)
    }

    fn tab_block() -> Block<'a> {
        Block::default().style(Style::default())
    }

    pub fn tab_chunk(&self) -> Rect {
        self.chunks()[window_layout_index::TAB]
    }
}

// Tab
impl<'a> Window<'a> {
    pub fn selected_tab_id(&self) -> &str {
        &self.tabs[self.selected_tab_index].id()
    }

    pub fn selected_tab(&self) -> &Tab {
        &self.tabs[self.selected_tab_index]
    }

    pub fn selected_tab_mut(&mut self) -> &mut Tab<'a> {
        &mut self.tabs[self.selected_tab_index]
    }

    pub fn select_tab(&mut self, index: usize) {
        let index = index - 1;
        if index < self.tabs.len() {
            self.selected_tab_index = index;
        }
    }

    pub fn select_next_tab(&mut self) {
        if self.tabs.len() - 1 <= self.selected_tab_index {
            self.selected_tab_index = 0;
        } else {
            self.selected_tab_index += 1;
        }
    }

    pub fn select_prev_tab(&mut self) {
        if 0 == self.selected_tab_index {
            self.selected_tab_index = self.tabs.len() - 1;
        } else {
            self.selected_tab_index -= 1;
        }
    }
}

// Pane
impl<'a> Window<'a> {
    pub fn pane(&self, id: &str) -> Option<&Pane<'a>> {
        for t in &self.tabs {
            let p = t.panes().iter().find(|p| p.id() == id);
            if p.is_some() {
                return p;
            }
        }
        None
    }
    pub fn pane_mut(&mut self, id: &str) -> Option<&mut Pane<'a>> {
        for t in &mut self.tabs {
            let p = t.panes_mut().iter_mut().find(|p| p.id() == id);
            if p.is_some() {
                return p;
            }
        }
        None
    }

    pub fn selected_pane_id(&self) -> &str {
        self.selected_tab().selected_pane_id()
    }

    pub fn select_next_pane(&mut self) {
        self.selected_tab_mut().next_pane();
    }

    pub fn select_prev_pane(&mut self) {
        self.selected_tab_mut().prev_pane();
    }

    pub fn pane_clear(&mut self, id: &str) {
        if let Some(pane) = self.pane_mut(id) {
            pane.clear();
        }
    }

    pub fn select_pane(&mut self, id: &str) {
        self.selected_tab_mut().select_pane(id)
    }
}

// フォーカスしているwidgetの状態変更
impl Window<'_> {
    pub fn select_next_item(&mut self) {
        self.selected_tab_mut().select_pane_next_item();
    }

    pub fn select_prev_item(&mut self) {
        self.selected_tab_mut().select_pane_prev_item();
    }

    pub fn select_first_item(&mut self) {
        self.selected_tab_mut().select_pane_first_item();
    }

    pub fn select_last_item(&mut self) {
        self.selected_tab_mut().select_pane_last_item();
    }

    pub fn scroll_up(&mut self) {
        let pane = self.selected_tab_mut().selected_pane_mut();
        let ch = pane.chunk();

        match pane.widget_mut() {
            Widget::List(list) => {
                list.prev();
            }
            Widget::Text(text) => {
                text.scroll_up(ch.height as u64);
            }
            Widget::Table(table) => {
                table.prev();
            }
        }
    }

    pub fn scroll_down(&mut self) {
        let pane = self.selected_tab_mut().selected_pane_mut();
        let ch = pane.chunk();

        match pane.widget_mut() {
            Widget::List(list) => {
                list.next();
            }
            Widget::Text(text) => {
                text.scroll_down(ch.height as u64);
            }
            Widget::Table(table) => {
                table.next();
            }
        }
    }
}

// Render
use window_layout_index::*;
impl<'a> Window<'a> {
    pub fn render<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        current_context: &str,
        current_namespace: &str,
    ) {
        self.render_tab(f);

        self.render_context(f, current_context, current_namespace);

        self.selected_tab_mut().render(f);

        self.render_status(f);
    }

    fn render_tab<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(self.widget(), self.tab_chunk());
    }

    fn render_context<B: Backend>(&mut self, f: &mut Frame<B>, ctx: &str, ns: &str) {
        let block = Block::default().style(Style::default());

        let text = format!("{}: {}", ns, ctx);
        let spans = Spans::from(text);
        let paragraph = Paragraph::new(spans).block(block);

        f.render_widget(paragraph, self.chunks()[CONTEXT]);
    }
    fn render_status<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(30)])
            .split(self.chunks()[STATUSBAR]);

        let datetime = datetime();

        let datetime = Spans::from(datetime);
        let block = Block::default().style(Style::default());
        let paragraph = Paragraph::new(datetime).block(block);

        f.render_widget(paragraph, chunks[0]);

        let widget = match self.selected_tab_id() {
            view_id::tab_pods => self.scroll_status(view_id::tab_pods_pane_logs),
            view_id::tab_configs => self.scroll_status(view_id::tab_configs_pane_raw_data),
            view_id::tab_event => self.scroll_status(view_id::tab_event_pane_event),
            view_id::tab_apis => self.scroll_status(view_id::tab_apis_pane_apis),
            _ => unreachable!(),
        };

        if let Some(widget) = widget {
            f.render_widget(widget.alignment(Alignment::Left), chunks[1]);
        }
    }

    fn scroll_status(&self, id: &str) -> Option<Paragraph<'a>> {
        if let Some(pane) = self.selected_tab().panes().iter().find(|p| p.id() == id) {
            let widget = pane.widget().as_text();
            let span = text_status((widget.state().selected_vertical(), widget.row_size()));

            let spans = Spans::from(span);
            let block = Block::default().style(Style::default());

            return Some(Paragraph::new(spans).block(block));
        }
        None
    }
}

fn datetime() -> Span<'static> {
    Span::raw(format!(
        " {}",
        Local::now().format("%Y年%m月%d日 %H時%M分%S秒")
    ))
}

fn text_status((current, rows): (u64, u64)) -> Span<'static> {
    Span::raw(format!("{}/{}", current, rows))
}

trait MouseEventTrait {
    fn on_click(&mut self);
}

// Mouse Event
impl Window<'_> {
    pub fn on_mouse_event(&mut self, ev: MouseEvent) {
        let pos = (ev.column, ev.row);

        if contains(self.tab_chunk(), pos) {
            self.on_click_tab(ev)
        } else if contains(self.chunks()[window_layout_index::CONTENTS], pos) {
            for pane in self.selected_tab_mut().panes_mut() {
                if contains(pane.chunk(), pos) {
                    pane.on_mouse_event(ev);
                    break;
                }
            }
        }
    }

    fn on_click_tab(&mut self, ev: MouseEvent) {
        if ev.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }

        let pos = mouse_pos(ev);

        let chunk = Self::tab_block().inner(self.tab_chunk());
        let divider_width = 1;

        let mut x = chunk.left();
        let y = chunk.top();
        let h = chunk.height;

        for (i, tab) in self.tabs.iter().enumerate() {
            let w = Self::tab_title_format(tab.title()).width() as u16;
            x = x.saturating_add(1);

            let title_chunk = Rect::new(x, y, w, h);

            if contains(title_chunk, pos) {
                self.select_tab(i + 1);
                break;
            }

            x = x
                .saturating_add(1)
                .saturating_add(w)
                .saturating_add(divider_width);
        }
    }
}

#[inline]
fn mouse_pos(ev: MouseEvent) -> (u16, u16) {
    (ev.column, ev.row)
}

#[inline]
fn contains(chunk: Rect, point: (u16, u16)) -> bool {
    let (px, py) = point;
    if (chunk.left() <= px && px <= chunk.right()) && (chunk.top() <= py && py <= chunk.bottom()) {
        true
    } else {
        false
    }
}
