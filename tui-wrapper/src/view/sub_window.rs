use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};

use super::{child_window_chunk, Pane};
use crate::widget::{WidgetItem, WidgetTrait};

#[derive(Clone)]
pub struct SubWindow<'a> {
    id: String,
    title: String,
    layout: Layout,
    chunk: Rect,
    panes: Vec<Pane<'a>>,
    selected_pane_index: usize,
    selectable_panes: Vec<usize>,
}

impl<'a> SubWindow<'a> {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        panes: Vec<Pane<'a>>,
        layout: Layout,
    ) -> Self {
        let selectable_panes = panes
            .iter()
            .enumerate()
            .filter(|(_i, p)| p.widget().selectable())
            .map(|(i, _)| i)
            .collect();

        Self {
            id: id.into(),
            title: title.into(),
            layout,
            chunk: Rect::default(),
            panes,
            selected_pane_index: 0,
            selectable_panes,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = child_window_chunk(80, 80, chunk);

        let chunks = self.layout.split(self.chunk);
        self.panes
            .iter_mut()
            .for_each(|p| p.update_chunk(chunks[p.chunk_index()]));
    }

    pub fn select_next_pane(&mut self) {
        if self.selectable_panes.len() - 1 <= self.selected_pane_index {
            self.selected_pane_index = 0;
        } else {
            self.selected_pane_index += 1;
        }
    }

    pub fn select_prev_pane(&mut self) {
        self.selected_pane_index = self.selected_pane_index.saturating_sub(1);
    }

    pub fn select_next_item(&mut self) {
        self.selected_pane_mut().select_next_item(1);
    }

    pub fn select_prev_item(&mut self) {
        self.selected_pane_mut().select_prev_item(1);
    }

    pub fn select_first_item(&mut self) {
        self.selected_pane_mut().select_first_item();
    }

    pub fn select_last_item(&mut self) {
        self.selected_pane_mut().select_last_item();
    }

    pub fn set_items(&mut self, id: &str, items: WidgetItem) {
        if let Some(pane) = self.panes.iter_mut().find(|p| p.id() == id) {
            pane.widget_mut().set_items(items)
        }
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        let selected_pane_index = self.selected_pane_index;

        f.render_widget(Clear, self.chunk);

        self.panes
            .iter_mut()
            .enumerate()
            .for_each(|(i, p)| p.render(f, i == selected_pane_index));
    }
}

impl<'a> SubWindow<'a> {
    fn selected_pane_mut(&mut self) -> &mut Pane<'a> {
        &mut self.panes[self.selected_pane_index]
    }

    pub fn selected_pane(&self) -> &Pane<'a> {
        &self.panes[self.selected_pane_index]
    }
}
