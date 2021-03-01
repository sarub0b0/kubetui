use super::{tab::*, Status, Type};

use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Tabs};

pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    selected_tab_index: usize,
    layout: Layout,
    chunk: Rect,
    status: Status,
}

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

        self.tabs
            .iter_mut()
            .for_each(|tab| tab.update_chunk(chunks[1]));
    }

    pub fn chunks(&self) -> Vec<Rect> {
        self.layout.split(self.chunk)
    }

    pub fn selected_tab_index(&self) -> usize {
        self.selected_tab_index
    }

    pub fn select_next_tab(&mut self) {
        if self.tabs.len() - 1 <= self.selected_tab_index {
            self.selected_tab_index = 0;
        } else {
            self.selected_tab_index += 1;
        }
    }

    pub fn selected_tab(&self) -> &Tab {
        &self.tabs[self.selected_tab_index]
    }

    pub fn selected_tab_mut(&mut self) -> &mut Tab<'a> {
        &mut self.tabs[self.selected_tab_index]
    }

    pub fn select_next_pane(&mut self) {
        self.tabs[self.selected_tab_index].next_pane();
    }

    pub fn select_prev_pane(&mut self) {
        self.tabs[self.selected_tab_index].prev_pane();
    }

    pub fn select_next_item(&mut self) {
        self.tabs[self.selected_tab_index]
            .selected_pane_mut()
            .next_item();
    }

    pub fn select_prev_item(&mut self) {
        self.tabs[self.selected_tab_index]
            .selected_pane_mut()
            .prev_item();
    }

    pub fn select_tab(&mut self, index: usize) {
        let index = index - 1;
        if index < self.tabs.len() {
            self.selected_tab_index = index;
        }
    }

    pub fn select_first_item(&mut self) {
        self.selected_tab_mut()
            .selected_pane_mut()
            .widget_mut()
            .first();
    }

    pub fn select_last_item(&mut self) {
        self.selected_tab_mut()
            .selected_pane_mut()
            .widget_mut()
            .last();
    }

    pub fn focus_pane_type(&self) -> Type {
        self.selected_tab().selected_pane().ty()
    }

    pub fn update_pod_status(&mut self, info: Vec<String>) {
        for t in &mut self.tabs {
            let pane = t.panes_mut().iter_mut().find(|p| p.ty() == Type::POD);

            if let Some(p) = pane {
                let pod = p.widget_mut().mut_pod().unwrap();
                pod.set_items(info.to_vec());
            }
        }
    }

    pub fn update_pod_logs(&mut self, logs: Vec<String>) {
        for t in &mut self.tabs {
            let pane = t.panes_mut().iter_mut().find(|p| p.ty() == Type::LOG);

            if let Some(p) = pane {
                let rect = p.chunk();
                let log = p.widget_mut().mut_log().unwrap();
                log.set_items(logs.to_vec());
                log.update_spans(rect.width);
                log.update_rows_size(rect.height);
            }
        }
    }

    pub fn selected_pod(&self) -> String {
        let pane = self.selected_tab().selected_pane();
        let selected_index = pane
            .widget()
            .pod()
            .unwrap()
            .state()
            .borrow()
            .selected()
            .unwrap();
        let split: Vec<&str> = pane.widget().pod().unwrap().items()[selected_index]
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
            .select(self.selected_tab_index())
            .highlight_style(Style::default().fg(Color::White).bg(Color::LightBlue))
    }

    pub fn tab_chunk(&self) -> Rect {
        self.chunks()[0]
    }

    pub fn log_status(&self) -> (u16, u16) {
        let mut curr = 0;
        let mut rows = 0;
        for t in &self.tabs {
            let pane = t.panes().iter().find(|p| p.ty() == Type::LOG);

            if let Some(p) = pane {
                let log = p.widget().log().unwrap();
                curr = log.selected();
                rows = log.row_size();
            }
        }
        (curr, rows)
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
            status: Status::new(),
        }
    }
}
