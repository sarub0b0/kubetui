use super::{pane::Pane, Popup, Type};

use crate::widget::*;
use tui::layout::{Constraint, Direction, Layout, Rect};

pub struct Tab<'a> {
    title: String,
    panes: Vec<Pane<'a>>,
    layout: Layout,
    selected_pane_index: usize,
    selectable_widgets: Vec<usize>,
    select_popup: bool,
    popup: Option<Popup<'a>>,
}

impl<'a> Tab<'a> {
    pub fn new(
        title: impl Into<String>,
        panes: Vec<Pane<'a>>,
        layout: Layout,
        popup: Option<Popup<'a>>,
    ) -> Self {
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
            select_popup: false,
            popup,
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
        if self.select_popup {
            return;
        }
        if self.selectable_widgets.len() - 1 <= self.selected_pane_index {
            self.selected_pane_index = 0;
        } else {
            self.selected_pane_index += 1;
        }
    }

    pub fn prev_pane(&mut self) {
        if self.select_popup {
            return;
        }
        if self.selected_pane_index == 0 {
            self.selected_pane_index = self.selectable_widgets.len() - 1;
        } else {
            self.selected_pane_index -= 1;
        }
    }

    pub fn select_pane_next_item(&mut self) {
        if self.select_popup {
            if let Some(popup) = &mut self.popup {
                popup.next_item()
            }
        } else {
            self.selected_pane_mut().next_item(1);
        }
    }

    pub fn select_pane_prev_item(&mut self) {
        if self.select_popup {
            if let Some(popup) = &mut self.popup {
                popup.prev_item()
            }
        } else {
            self.selected_pane_mut().prev_item(1);
        }
    }

    pub fn select_pane_first_item(&mut self) {
        if self.select_popup {
            if let Some(popup) = &mut self.popup {
                popup.first_item()
            }
        } else {
            self.selected_pane_mut().widget_mut().select_first();
        }
    }

    pub fn select_pane_last_item(&mut self) {
        if self.select_popup {
            if let Some(popup) = &mut self.popup {
                popup.last_item()
            }
        } else {
            self.selected_pane_mut().widget_mut().select_last();
        }
    }

    pub fn selected_pane_type(&self) -> Type {
        self.selected_pane().ty()
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

    pub fn select_popup(&mut self) {
        if let None = self.popup {
            return;
        }
        self.select_popup = true;
    }

    pub fn unselect_popup(&mut self) {
        if let None = self.popup {
            return;
        }
        self.select_popup = false;
    }

    pub fn selected_popup(&self) -> bool {
        if let None = self.popup {
            return false;
        }
        self.select_popup
    }

    pub fn update_popup_chunk(&mut self, chunk: Rect) {
        if let Some(popup) = &mut self.popup {
            popup.update_chunk(chunk);
        }
    }

    pub fn popup(&self) -> &Option<Popup<'a>> {
        &self.popup
    }
    pub fn popup_mut(&mut self) -> &mut Option<Popup<'a>> {
        &mut self.popup
    }
}
