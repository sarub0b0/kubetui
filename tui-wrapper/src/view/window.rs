use super::{tab::*, Pane, Popup};
use crate::widget::Widget;

use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::Spans;
use tui::widgets::{Block, Tabs};

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
    pub const CONTEXT: usize = 1;
    pub const CONTENTS: usize = 2;
    pub const STATUSBAR: usize = 3;
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
                        Constraint::Length(2),
                        Constraint::Length(1),
                        Constraint::Min(0),
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
        self.chunks()[window_layout_index::TAB]
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
