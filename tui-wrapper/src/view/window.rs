use super::{tab::*, Pane, Popup};
use crate::widget::Widget;

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};

pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    selected_tab_index: usize,
    layout: Layout,
    chunk: Rect,
    popup: Popup<'a>,
    selected_popup: bool,
}

pub mod window_layout_index {
    pub const TAB: usize = 0;
    pub const CONTEXT: usize = 2;
    pub const CONTENTS: usize = 3;
    pub const STATUSBAR: usize = 4;
}

// Window
impl<'a> Window<'a> {
    pub fn new(tabs: Vec<Tab<'a>>, popup: Popup<'a>) -> Self {
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
            popup,
            selected_popup: false,
        }
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let chunks = self.layout.split(chunk);

        self.tabs.iter_mut().for_each(|tab| {
            tab.update_chunk(chunks[window_layout_index::CONTENTS]);
        });
        self.popup.update_chunk(chunk);
    }

    pub fn chunks(&self) -> Vec<Rect> {
        self.layout.split(self.chunk)
    }

    pub fn widget(&self) -> Tabs {
        let titles: Vec<Spans> = self
            .tabs
            .iter()
            .map(|t| Spans::from(format!(" {} ", t.title())))
            .collect();

        let block = Block::default().style(Style::default());

        Tabs::new(titles)
            .block(block)
            .select(self.selected_tab_index)
            .highlight_style(Style::default().fg(Color::White).bg(Color::LightBlue))
    }

    pub fn tab_chunk(&self) -> Rect {
        let chunk = self.chunks()[window_layout_index::TAB];
        if chunk.height == 0 {
            Rect::new(chunk.x, chunk.y, chunk.width, 1)
        } else {
            chunk
        }
    }
}

// Tab
impl<'a> Window<'a> {
    pub fn selected_tab(&self) -> &Tab {
        &self.tabs[self.selected_tab_index]
    }

    pub fn selected_tab_mut(&mut self) -> &mut Tab<'a> {
        &mut self.tabs[self.selected_tab_index]
    }

    pub fn select_tab(&mut self, index: usize) {
        if self.selected_popup {
            return;
        }
        let index = index - 1;
        if index < self.tabs.len() {
            self.selected_tab_index = index;
        }
    }

    pub fn select_next_tab(&mut self) {
        if self.selected_popup {
            return;
        }
        if self.tabs.len() - 1 <= self.selected_tab_index {
            self.selected_tab_index = 0;
        } else {
            self.selected_tab_index += 1;
        }
    }

    pub fn select_prev_tab(&mut self) {
        if self.selected_popup {
            return;
        }
        if 0 == self.selected_tab_index {
            self.selected_tab_index = self.tabs.len() - 1;
        } else {
            self.selected_tab_index -= 1;
        }
    }
}

// Pane
impl<'a> Window<'a> {
    pub fn pane(&self, id: impl Into<String>) -> Option<&Pane<'a>> {
        let id = id.into();
        for t in &self.tabs {
            let p = t.panes().iter().find(|p| p.id() == id);
            if p.is_some() {
                return p;
            }
        }
        None
    }
    pub fn pane_mut(&mut self, id: impl Into<String>) -> Option<&mut Pane<'a>> {
        let id = id.into();
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
        if self.selected_popup {
            return;
        }
        self.selected_tab_mut().next_pane();
    }

    pub fn select_prev_pane(&mut self) {
        if self.selected_popup {
            return;
        }
        self.selected_tab_mut().prev_pane();
    }
}

// フォーカスしているwidgetの状態変更
impl Window<'_> {
    pub fn select_next_item(&mut self) {
        if self.selected_popup {
            self.popup.next_item();
            return;
        }
        self.selected_tab_mut().select_pane_next_item();
    }

    pub fn select_prev_item(&mut self) {
        if self.selected_popup {
            self.popup.prev_item();
            return;
        }
        self.selected_tab_mut().select_pane_prev_item();
    }

    pub fn select_first_item(&mut self) {
        if self.selected_popup {
            self.popup.first_item();
            return;
        }
        self.selected_tab_mut().select_pane_first_item();
    }

    pub fn select_last_item(&mut self) {
        if self.selected_popup {
            self.popup.last_item();
            return;
        }
        self.selected_tab_mut().select_pane_last_item();
    }

    pub fn scroll_up(&mut self) {
        if self.selected_popup {
            return;
        }
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
        if self.selected_popup {
            return;
        }
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

// Popup
impl<'a> Window<'a> {
    pub fn popup(&self) -> &Popup {
        &self.popup
    }

    pub fn selected_popup(&self) -> bool {
        self.selected_popup
    }

    pub fn select_popup(&mut self) {
        self.selected_popup = true;
    }
    pub fn unselect_popup(&mut self) {
        self.selected_popup = false;
    }

    pub fn popup_mut(&mut self) -> &mut Popup<'a> {
        &mut self.popup
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

        if self.selected_popup() {
            self.popup_mut().render(f);
        }
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
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(self.chunks()[STATUSBAR]);

        let datetime = datetime();

        let datetime = Spans::from(datetime);
        let block = Block::default().style(Style::default());
        let paragraph = Paragraph::new(datetime).block(block);

        f.render_widget(paragraph, chunks[0]);

        if let Some(p) = self.scroll_status("logs") {
            f.render_widget(p, chunks[1]);
        }

        if let Some(p) = self.scroll_status("configs-raw") {
            f.render_widget(p, chunks[1]);
        }

        if let Some(p) = self.scroll_status("event") {
            f.render_widget(p, chunks[1]);
        }
    }

    fn scroll_status(&self, id: &str) -> Option<Paragraph<'a>> {
        if let Some(pane) = self.selected_tab().panes().iter().find(|p| p.id() == id) {
            let widget = pane.widget().text();
            let span = match widget {
                Some(t) => text_status((t.selected(), t.row_size())),
                None => text_status((0, 0)),
            };

            let spans = Spans::from(span);
            let block = Block::default().style(Style::default());

            return Some(
                Paragraph::new(spans)
                    .block(block)
                    .alignment(Alignment::Right),
            );
        }
        None
    }
}

use chrono::Local;
fn datetime() -> Span<'static> {
    Span::raw(format!(
        " {}",
        Local::now().format("%Y年%m月%d日 %H時%M分%S秒")
    ))
}

fn text_status((current, rows): (u64, u64)) -> Span<'static> {
    Span::raw(format!("{}/{}", current, rows))
}
