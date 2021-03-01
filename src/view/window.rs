use super::{tab::*, Pane, Status, Type};

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
    status: Status,
    popup: Popup<'a>,
}

struct Popup<'a> {
    items: Vec<String>,
    list_item: Vec<ListItem<'a>>,
    state: Rc<RefCell<ListState>>,
}

impl Default for Popup<'_> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            list_item: Vec::new(),
            state: Rc::new(RefCell::new(ListState::default())),
        }
    }
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
        let pane = self.pane_mut(Type::POD);

        if let Some(p) = pane {
            let pod = p.widget_mut().mut_pod().unwrap();
            pod.set_items(info.to_vec());
        }
    }

    pub fn update_pod_logs(&mut self, logs: Vec<String>) {
        let pane = self.pane_mut(Type::LOG);
        if let Some(p) = pane {
            let rect = p.chunk();
            let log = p.widget_mut().mut_log().unwrap();
            log.set_items(logs.to_vec());
            log.update_spans(rect.width);
            log.update_rows_size(rect.height);
        }
    }

    fn pane_mut(&mut self, ty: Type) -> Option<&mut Pane<'a>> {
        for t in &mut self.tabs {
            return t.panes_mut().iter_mut().find(|p| p.ty() == ty);
        }
        None
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
        match self.selected_tab().selected_pane().widget().log() {
            Some(log) => (log.selected(), log.row_size()),
            None => (0, 0),
        }
    }

    pub fn update_wrap(&mut self) {
        let pane = self.pane_mut(Type::LOG);
        if let Some(p) = pane {
            let rect = p.chunk();
            let log = p.widget_mut().mut_log().unwrap();
            log.update_spans(rect.width);
            log.update_rows_size(rect.height);
        }
    }

    pub fn scroll_up(&mut self) {
        let pane = self.selected_tab_mut().selected_pane_mut();
        if pane.ty() != Type::LOG {
            return;
        }

        let ch = pane.chunk();
        if let Some(log) = pane.widget_mut().mut_log() {
            (0..ch.height).for_each(|_| log.prev());
        }
    }

    pub fn scroll_down(&mut self) {
        let pane = self.selected_tab_mut().selected_pane_mut();
        if pane.ty() != Type::LOG {
            return;
        }

        let ch = pane.chunk();
        if let Some(log) = pane.widget_mut().mut_log() {
            (0..ch.height).for_each(|_| log.next());
        }
    }

    pub fn setup_popup(&mut self, items: Option<Vec<String>>) {
        if let Some(items) = items {
            self.popup.items = items;
            self.popup.list_item = ["Item 0", "Item 1", "Item 2"]
                .iter()
                .cloned()
                .map(|i| ListItem::new(i))
                .collect();
        }
    }

    pub fn popup(&self) -> (List, Rc<RefCell<ListState>>, Rect) {
        let h = 20;
        let w = 20;
        let chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - h) / 2),
                Constraint::Percentage(h),
                Constraint::Percentage((100 - h) / 2),
            ])
            .split(self.chunk);

        let chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - w) / 2),
                Constraint::Percentage(w),
                Constraint::Percentage((100 - w) / 2),
            ])
            .split(chunk[1])[1];

        (
            List::new(self.popup.list_item.clone())
                .block(
                    Block::default()
                        .title("Namespace")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double),
                )
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
            self.popup.state.clone(),
            chunk,
        )
    }

    pub fn drawable_popup(&self) -> bool {
        0 < self.popup.items.len()
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
            popup: Popup::default(),
        }
    }
}
