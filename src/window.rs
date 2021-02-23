use super::event::*;
use super::widget::*;

use tui::layout::{Constraint, Direction, Layout, Rect};

pub struct Window {
    tabs: Vec<Tab>,
    selected_tab_index: usize,
    layout: Layout,
}

pub struct Tab {
    title: String,
    panes: Vec<Pane>,
    layout: Layout,
    selected_pane_index: usize,
    selectable_widgets: Vec<usize>,
}

pub struct Pane {
    widget: Widgets,
    chunk_index: usize,
    title: String,
    ty: Type,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Type {
    NONE,
    LOG,
    POD,
}

impl Window {
    pub fn new(tabs: Vec<Tab>) -> Self {
        Self {
            tabs,
            ..Window::default()
        }
    }

    pub fn chunks(&self, window_size: Rect) -> Vec<Rect> {
        self.layout.split(window_size)
    }

    pub fn tabs(&self) -> &Vec<Tab> {
        &self.tabs
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

    pub fn select_next_pane(&mut self) {
        self.tabs[self.selected_tab_index].next_pane();
    }

    pub fn select_prev_pane(&mut self) {
        self.tabs[self.selected_tab_index].prev_pane();
    }

    pub fn select_next_item(&mut self) {
        self.tabs[self.selected_tab_index]
            .selected_mut_pane()
            .next_item();
    }

    pub fn select_prev_item(&mut self) {
        self.tabs[self.selected_tab_index]
            .selected_mut_pane()
            .prev_item();
    }

    pub fn select_tab(&mut self, index: usize) {
        let index = index - 1;
        if index < self.tabs.len() {
            self.selected_tab_index = index;
        }
    }

    pub fn select_first_item(&self) {
        self.selected_tab().selected_pane().widget().first();
    }

    pub fn select_last_item(&self) {
        self.selected_tab().selected_pane().widget().last();
    }

    pub fn focus_pane_type(&self) -> Type {
        self.selected_tab().selected_pane().ty
    }

    pub fn update_pod_status(&mut self, info: &Vec<String>) {
        for t in &mut self.tabs {
            let pane = t.panes.iter_mut().find(|p| p.ty == Type::POD);

            if let Some(p) = pane {
                let pod = p.widget.mut_pod().unwrap();
                pod.set_items(info.to_vec());
            }
        }
    }

    pub fn update_pod_logs(&mut self, logs: &Vec<String>) {
        for t in &mut self.tabs {
            let pane = t.panes.iter_mut().find(|p| p.ty == Type::LOG);

            if let Some(p) = pane {
                let log = p.widget.mut_log().unwrap();
                log.set_items(logs.to_vec());
            }
        }
    }

    pub fn reset_pod_logs(&mut self) {
        for t in &mut self.tabs {
            let pane = t.panes.iter_mut().find(|p| p.ty == Type::LOG);

            if let Some(p) = pane {
                // p.widget.unselect();
                // p.widget.set_items(vec![]);
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
}

impl Default for Window {
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
        }
    }
}

impl Tab {
    pub fn new(title: String, panes: Vec<Pane>, layout: Layout) -> Self {
        let selectable_widgets = panes
            .iter()
            .enumerate()
            .filter(|&(_, p)| p.widget.selectable())
            .map(|(i, _)| i)
            .collect();

        Self {
            title,
            panes,
            layout,
            selectable_widgets,
            selected_pane_index: 0,
        }
    }
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn chunks(&self, tab_size: Rect) -> Vec<Rect> {
        self.layout.split(tab_size)
    }

    pub fn panes(&self) -> &Vec<Pane> {
        &self.panes
    }

    pub fn next_pane(&mut self) {
        if self.selectable_widgets.len() - 1 <= self.selected_pane_index {
            self.selected_pane_index = 0;
        } else {
            self.selected_pane_index += 1;
        }
    }

    pub fn prev_pane(&mut self) {
        if self.selected_pane_index == 0 {
            self.selected_pane_index = self.selectable_widgets.len() - 1;
        } else {
            self.selected_pane_index -= 1;
        }
    }

    pub fn selected_mut_pane(&mut self) -> &mut Pane {
        &mut self.panes[self.selected_pane_index]
    }
    pub fn selected_pane(&self) -> &Pane {
        &self.panes[self.selected_pane_index]
    }
}

impl Pane {
    pub fn new(title: String, widget: Widgets, chunk_index: usize, ty: Type) -> Self {
        Self {
            title,
            widget,
            chunk_index,
            ty,
        }
    }

    pub fn widget(&self) -> &Widgets {
        &self.widget
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn chunk_index(&self) -> usize {
        self.chunk_index
    }

    pub fn next_item(&mut self) {
        self.widget.next()
    }

    pub fn prev_item(&mut self) {
        self.widget.prev()
    }

    pub fn selected(&self, rhs: &Pane) -> bool {
        return std::ptr::eq(self, rhs);
    }

    pub fn ty(&self) -> Type {
        self.ty
    }
}
