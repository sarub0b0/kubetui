use super::pane::Pane;

use crate::widget::*;
use tui::{
    backend::Backend,
    layout::{Layout, Rect},
    Frame,
};

pub struct Tab<'a> {
    title: String,
    panes: Vec<Pane<'a>>,
    layout: Layout,
    selected_pane_index: usize,
    selectable_widgets: Vec<usize>,
}

impl<'a> Tab<'a> {
    pub fn new(title: impl Into<String>, panes: Vec<Pane<'a>>, layout: Layout) -> Self {
        let selectable_widgets = panes
            .iter()
            .enumerate()
            .filter(|&(_, p)| p.widget().selectable())
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

    pub fn panes(&self) -> &Vec<Pane<'a>> {
        &self.panes
    }

    pub fn panes_mut(&mut self) -> &mut Vec<Pane<'a>> {
        &mut self.panes
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

    pub fn select_pane_next_item(&mut self) {
        self.selected_pane_mut().next_item(1);
    }

    pub fn select_pane_prev_item(&mut self) {
        self.selected_pane_mut().prev_item(1);
    }

    pub fn select_pane_first_item(&mut self) {
        self.selected_pane_mut().widget_mut().select_first();
    }

    pub fn select_pane_last_item(&mut self) {
        self.selected_pane_mut().widget_mut().select_last();
    }

    pub fn selected_pane_id(&self) -> &str {
        self.selected_pane().id()
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
            .for_each(|pane| pane.update_chunk(chunks[pane.chunk_index()]));
    }
}

impl Tab<'_> {
    pub fn render<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let selected_pane_index = self.selected_pane_index;

        self.panes
            .iter_mut()
            .enumerate()
            .for_each(|(i, p)| p.render(f, i == selected_pane_index));
    }
}
