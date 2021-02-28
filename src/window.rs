use super::event::*;
use super::widget::*;

use tui::layout::{Constraint, Direction, Layout, Rect};

pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    selected_tab_index: usize,
    layout: Layout,
    chunk: Rect,
}

pub struct Tab<'a> {
    title: String,
    panes: Vec<Pane<'a>>,
    layout: Layout,
    selected_pane_index: usize,
    selectable_widgets: Vec<usize>,
}

pub struct Pane<'a> {
    widget: Widgets<'a>,
    chunk_index: usize,
    title: String,
    ty: Type,
    chunk: Rect,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Type {
    NONE,
    LOG,
    POD,
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
        self.selected_tab().selected_pane().ty
    }

    pub fn update_pod_status(&mut self, info: Vec<String>) {
        for t in &mut self.tabs {
            let pane = t.panes.iter_mut().find(|p| p.ty == Type::POD);

            if let Some(p) = pane {
                let pod = p.widget.mut_pod().unwrap();
                pod.set_items(info.to_vec());
            }
        }
    }

    pub fn update_pod_logs(&mut self, logs: Vec<String>) {
        for t in &mut self.tabs {
            let pane = t.panes.iter_mut().find(|p| p.ty == Type::LOG);

            if let Some(p) = pane {
                let log = p.widget.mut_log().unwrap();
                log.set_items(logs.to_vec());
                log.update_rows_size(p.chunk.width, p.chunk.height);
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

impl<'a> Tab<'a> {
    pub fn new(title: impl Into<String>, panes: Vec<Pane<'a>>, layout: Layout) -> Self {
        let selectable_widgets = panes
            .iter()
            .enumerate()
            .filter(|&(_, p)| p.widget.selectable())
            .map(|(i, _)| i)
            .collect();

        Self {
            title: title.into(),
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

    pub fn selected_pane_mut(&mut self) -> &mut Pane<'a> {
        &mut self.panes[self.selected_pane_index]
    }
    pub fn selected_pane(&self) -> &Pane {
        &self.panes[self.selected_pane_index]
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        let chunks = self.layout.split(chunk);
        self.panes
            .iter_mut()
            .for_each(|pane| pane.update_chunk(chunks[pane.chunk_index]));
    }
}

impl<'a> Pane<'a> {
    pub fn new(
        title: impl Into<String>,
        widget: Widgets<'a>,
        chunk_index: usize,
        ty: Type,
    ) -> Self {
        Self {
            title: title.into(),
            widget,
            chunk_index,
            ty,
            chunk: Rect::default(),
        }
    }

    pub fn widget(&self) -> &Widgets {
        &self.widget
    }

    pub fn widget_mut(&mut self) -> &mut Widgets<'a> {
        &mut self.widget
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

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }

    pub fn chunk(&self) -> Rect {
        self.chunk
    }
}
