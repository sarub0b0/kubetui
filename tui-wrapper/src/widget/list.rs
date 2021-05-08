use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    Frame,
};

use tui::widgets::{self, Block, ListItem, ListState};

use super::{RenderTrait, WidgetItem, WidgetTrait};

#[derive(Debug, Clone)]
pub struct List<'a> {
    items: Vec<String>,
    state: ListState,
    list_item: Vec<ListItem<'a>>,
}

impl Default for List<'_> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            state: ListState::default(),
            list_item: Vec::new(),
        }
    }
}

impl<'a> List<'a> {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        let list_item = items.iter().cloned().map(ListItem::new).collect();

        Self {
            items,
            state,
            list_item,
        }
    }

    pub fn state(&self) -> &ListState {
        &self.state
    }

    pub fn next(&mut self) {
        self.select_next(1);
    }

    pub fn prev(&mut self) {
        self.select_prev(1);
    }

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn widget(&self, block: Block<'a>) -> widgets::List<'a> {
        widgets::List::new(self.list_item.clone())
            .block(block)
            .style(Style::default())
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    }

    fn set_listitem(&mut self) {
        self.list_item = self.items.iter().cloned().map(ListItem::new).collect();
    }
}

impl WidgetTrait for List<'_> {
    fn selectable(&self) -> bool {
        true
    }
    fn select_next(&mut self, index: usize) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if self.items.len() - 1 <= i {
                    self.items.len() - 1
                } else {
                    i + index
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn select_prev(&mut self, index: usize) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i < index {
                    0
                } else {
                    i.saturating_sub(1)
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
    }

    fn select_first(&mut self) {
        self.state.select(Some(0));
    }

    fn select_last(&mut self) {
        if self.items.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(Some(self.items.len() - 1))
        }
    }

    fn set_items(&mut self, items: WidgetItem) {
        let items = items.array();
        let old_len = self.items.len();

        match items.len() {
            0 => self.state.select(None),
            new_len if new_len < old_len => {
                let i = self.state.selected();
                if i == Some(old_len - 1) {
                    self.state.select(Some(new_len - 1));
                }
            }
            _ => {
                if self.state.selected() == None {
                    self.state.select(Some(0))
                }
            }
        }
        self.items = items;

        self.set_listitem();
    }

    fn update_area(&mut self, _area: Rect) {}
    fn clear(&mut self) {}

    fn get_item(&self) -> Option<WidgetItem> {
        let index = self.state.selected();
        match index {
            Some(i) => Some(WidgetItem::Single(self.items[i].clone())),
            None => None,
        }
    }
}

impl RenderTrait for List<'_> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>, block: Block, chunk: Rect) {
        f.render_stateful_widget(self.widget(block), chunk, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_index() {
        let list = List::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        assert_eq!(Some(0), list.selected())
    }

    #[test]
    fn two_prev_is_selected_last_index() {
        let mut list = List::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.prev();
        list.prev();
        assert_eq!(Some(0), list.selected())
    }
    #[test]
    fn one_next_is_selected_second_index() {
        let mut list = List::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.next();
        assert_eq!(Some(1), list.selected())
    }

    #[test]
    fn last_next_is_selected_first_index() {
        let mut list = List::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.next();
        list.next();
        list.next();
        assert_eq!(Some(2), list.selected())
    }
}
