use super::{pane::Pane, Type};

use crate::widget::*;
use tui::layout::{Constraint, Direction, Layout, Rect};

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

    pub fn panes(&self) -> &Vec<Pane> {
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
