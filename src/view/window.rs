use super::{tab::*, Pane, Popup};
use crate::widget::WidgetTrait;

use std::cell::RefCell;
use std::rc::Rc;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Tabs};

pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    selected_tab_index: usize,
    layout: Layout,
    chunk: Rect,
}

// Private
impl<'a> Window<'a> {}

// Public
impl<'a> Window<'a> {
    pub fn new(tabs: Vec<Tab<'a>>) -> Self {
        Self {
            tabs,
            ..Window::default()
        }
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let chunks = self.layout.split(chunk);

        self.tabs.iter_mut().for_each(|tab| {
            tab.update_chunk(chunks[1]);
            tab.update_popup_chunk(chunk);
        });
    }

    pub fn chunks(&self) -> Vec<Rect> {
        self.layout.split(self.chunk)
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

    pub fn select_next_pane(&mut self) {
        self.selected_tab_mut().next_pane();
    }

    pub fn select_prev_pane(&mut self) {
        self.selected_tab_mut().prev_pane();
    }

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

    pub fn selected_pane_id(&self) -> &str {
        self.selected_tab().selected_pane_id()
    }

    pub fn pane_mut(&mut self, id: impl Into<String>) -> Option<&mut Pane<'a>> {
        let id = id.into();
        for t in &mut self.tabs {
            return t.panes_mut().iter_mut().find(|p| p.id() == id);
        }
        None
    }

    pub fn selected_pod(&self) -> String {
        let pane = self.selected_tab().selected_pane();
        let selected_index = pane
            .widget()
            .list()
            .unwrap()
            .state()
            .borrow()
            .selected()
            .unwrap();
        let split: Vec<&str> = pane.widget().list().unwrap().items()[selected_index]
            .split(' ')
            .collect();
        split[0].to_string()
    }

    pub fn tabs(&self) -> Tabs {
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
        self.chunks()[0]
    }

    pub fn log_status(&self) -> (u16, u16) {
        match self.selected_tab().selected_pane().widget().text() {
            Some(log) => (log.selected(), log.row_size()),
            None => (0, 0),
        }
    }

    pub fn update_wrap(&mut self) {
        let pane = self.pane_mut("logs");
        if let Some(p) = pane {
            let rect = p.chunk();
            let log = p.widget_mut().text_mut().unwrap();
            log.update_spans(rect.width);
            log.update_rows_size(rect.height);
        }
    }

    pub fn scroll_up(&mut self) {
        let pane = self.selected_tab_mut().selected_pane_mut();
        if pane.id() != "logs" {
            return;
        }

        let ch = pane.chunk();
        if let Some(log) = pane.widget_mut().text_mut() {
            (0..ch.height).for_each(|_| log.prev());
        }
    }

    pub fn scroll_down(&mut self) {
        let pane = self.selected_tab_mut().selected_pane_mut();
        if pane.id() != "logs" {
            return;
        }

        let ch = pane.chunk();
        if let Some(log) = pane.widget_mut().text_mut() {
            (0..ch.height).for_each(|_| log.next());
        }
    }

    pub fn setup_namespaces_popup(&mut self, items: Option<Vec<String>>) {
        if let Some(items) = items {
            let popup = self.selected_tab_mut().popup_mut();
            if let Some(popup) = popup {
                let ns = popup.widget_mut().list_mut();
                if let Some(ns) = ns {
                    ns.set_items(items);
                }
            }
        }
    }

    pub fn popup(&self) -> Option<&Popup> {
        self.selected_tab().popup()
    }

    pub fn selected_popup(&self) -> bool {
        self.selected_tab().selected_popup()
    }

    pub fn select_popup(&mut self) {
        self.selected_tab_mut().select_popup();
    }
    pub fn unselect_popup(&mut self) {
        self.selected_tab_mut().unselect_popup();
    }
}

impl Default for Window<'_> {
    fn default() -> Self {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ]
                .as_ref(),
            );

        Self {
            tabs: Vec::new(),
            selected_tab_index: 0,
            layout,
            chunk: Rect::default(),
        }
    }
}
