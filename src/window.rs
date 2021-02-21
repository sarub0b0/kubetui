use super::event::*;

use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::widgets::ListState;

use std::cell::RefCell;

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
    widget: Box<dyn Widget>,
    chunk_index: usize,
    title: String,
    ty: Type,
}

pub struct Pods {
    items: Vec<String>,
    state: RefCell<ListState>,
}

pub struct Logs {
    items: Vec<String>,
    state: RefCell<ListState>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Type {
    NONE,
    LOG,
    POD,
}

pub trait Widget {
    fn next(&self);
    fn prev(&self);
    fn selectable(&self) -> bool;
    fn set_items(&mut self, items: Vec<String>);
    fn items(&self) -> &Vec<String>;
    fn add_item(&mut self, item: &String);
    fn list_state(&self) -> &RefCell<ListState>;
    fn unselect(&self);
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

    pub fn focus_pane_type(&self) -> Type {
        self.selected_tab().selected_pane().ty
    }
    pub fn update_pod_status(&mut self, info: &Vec<String>) {
        for t in &mut self.tabs {
            let pane = t.panes.iter_mut().find(|p| p.ty == Type::POD);

            if let Some(p) = pane {
                p.widget.set_items(info.to_vec());
            }
        }
    }

    pub fn update_pod_logs(&mut self, logs: &String) {
        // if let Some(logs) = logs {
        for t in &mut self.tabs {
            let pane = t.panes.iter_mut().find(|p| p.ty == Type::LOG);

            if let Some(p) = pane {
                p.widget.add_item(logs);
            }
        }
        // }
    }

    pub fn reset_pod_logs(&mut self) {
        for t in &mut self.tabs {
            let pane = t.panes.iter_mut().find(|p| p.ty == Type::LOG);

            if let Some(p) = pane {
                p.widget.unselect();
                p.widget.set_items(vec![]);
            }
        }
    }

    pub fn selected_pod(&self) -> String {
        let pane = self.selected_tab().selected_pane();
        let selected_index = pane.widget().list_state().borrow().selected().unwrap();
        let split: Vec<&str> = pane.widget().items()[selected_index].split(' ').collect();
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
    pub fn new(title: String, widget: Box<dyn Widget>, chunk_index: usize, ty: Type) -> Self {
        Self {
            title,
            widget,
            chunk_index,
            ty,
        }
    }

    pub fn widget(&self) -> &Box<dyn Widget> {
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

impl Pods {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        if 0 < items.len() {
            state.select(Some(0));
        }

        Self {
            items,
            state: RefCell::new(state),
        }
    }
    pub fn unselect(&self) {
        self.state.borrow_mut().select(None);
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.borrow().selected()
    }
    pub fn state(&self) -> &RefCell<ListState> {
        &self.state
    }
}

impl Widget for Pods {
    fn next(&self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if self.items.len() - 1 <= i {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.state.borrow_mut().select(Some(i));
    }

    fn prev(&self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.state.borrow_mut().select(Some(i));
    }

    fn selectable(&self) -> bool {
        true
    }
    fn set_items(&mut self, items: Vec<String>) {
        match items.len() {
            0 => self.state.borrow_mut().select(None),
            len if len < self.items.len() => self.state.borrow_mut().select(Some(len - 1)),
            _ => {}
        }
        self.items = items;
    }

    fn add_item(&mut self, item: &String) {
        self.items.push(item.clone());
    }

    fn items(&self) -> &Vec<String> {
        &self.items
    }

    fn list_state(&self) -> &RefCell<ListState> {
        &self.state
    }
    fn unselect(&self) {
        self.state.borrow_mut().select(None);
    }
}

impl Logs {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        if 0 < items.len() {
            state.select(Some(0));
        }

        Self {
            items,
            state: RefCell::new(state),
        }
    }
    pub fn selected(&self) -> Option<usize> {
        self.state.borrow().selected()
    }
    pub fn state(&self) -> &RefCell<ListState> {
        &self.state
    }
}

impl Widget for Logs {
    fn next(&self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if self.items.len() - 1 <= i {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.state.borrow_mut().select(Some(i));
    }

    fn prev(&self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.state.borrow_mut().select(Some(i));
    }

    fn selectable(&self) -> bool {
        true
    }
    fn set_items(&mut self, items: Vec<String>) {
        match items.len() {
            0 => self.state.borrow_mut().select(None),
            len if len < self.items.len() => self.state.borrow_mut().select(Some(len - 1)),
            _ => {}
        }
        self.items = items;
    }

    fn items(&self) -> &Vec<String> {
        &self.items
    }

    fn add_item(&mut self, item: &String) {
        self.items.push(item.clone());
        self.state.borrow_mut().select(Some(self.items.len() - 1));
    }

    fn list_state(&self) -> &RefCell<ListState> {
        &self.state
    }
    fn unselect(&self) {
        self.state.borrow_mut().select(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_index() {
        let list = Pods::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        assert_eq!(Some(0), list.selected())
    }

    #[test]
    fn two_prev_is_selected_last_index() {
        let list = Pods::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.prev();
        list.prev();
        assert_eq!(Some(1), list.selected())
    }
    #[test]
    fn one_next_is_selected_second_index() {
        let list = Pods::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.next();
        assert_eq!(Some(1), list.selected())
    }

    #[test]
    fn last_next_is_selected_first_index() {
        let list = Pods::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.next();
        list.next();
        list.next();
        assert_eq!(Some(0), list.selected())
    }
}
