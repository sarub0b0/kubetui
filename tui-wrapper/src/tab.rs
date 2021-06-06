use super::widget::*;

use tui::{
    backend::Backend,
    layout::{Layout, Rect},
    Frame,
};

pub struct WidgetData<'a> {
    pub chunk_index: usize,
    pub widget: Widget<'a>,
}

pub struct Tab<'a> {
    id: String,
    title: String,
    panes: Vec<WidgetData<'a>>,
    layout: Layout,
    selected_pane_index: usize,
    selectable_panes: Vec<usize>,
}

impl<'a> Tab<'a> {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        panes: Vec<WidgetData<'a>>,
        layout: Layout,
    ) -> Self {
        let selectable_widgets = panes
            .iter()
            .enumerate()
            .filter(|&(_, w)| w.widget.selectable())
            .map(|(i, _)| i)
            .collect();

        Self {
            id: id.into(),
            title: title.into(),
            panes,
            layout,
            selectable_panes: selectable_widgets,
            selected_pane_index: 0,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn chunks(&self, tab_size: Rect) -> Vec<Rect> {
        self.layout.split(tab_size)
    }

    pub fn panes(&self) -> Vec<&Widget<'a>> {
        self.panes.iter().map(|w| &w.widget).collect()
    }

    pub fn panes_mut(&mut self) -> Vec<&mut Widget<'a>> {
        self.panes
            .iter_mut()
            .map(|w: &mut WidgetData| &mut w.widget)
            .collect()
    }

    pub fn next_pane(&mut self) {
        if self.selectable_panes.len() - 1 <= self.selected_pane_index {
            self.selected_pane_index = 0;
        } else {
            self.selected_pane_index += 1;
        }
    }

    pub fn prev_pane(&mut self) {
        if self.selected_pane_index == 0 {
            self.selected_pane_index = self.selectable_panes.len() - 1;
        } else {
            self.selected_pane_index -= 1;
        }
    }

    pub fn selected_pane_id(&self) -> &str {
        self.selected_pane().id()
    }

    pub fn selected_pane_mut(&mut self) -> &mut Widget<'a> {
        &mut self.panes[self.selected_pane_index].widget
    }

    pub fn selected_pane(&self) -> &Widget {
        &self.panes[self.selected_pane_index].widget
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        let chunks = self.layout.split(chunk);
        self.panes
            .iter_mut()
            .for_each(|pane| pane.widget.update_chunk(chunks[pane.chunk_index]));
    }

    pub fn select_pane(&mut self, id: &str) {
        if let Some((index, _)) = self
            .panes
            .iter()
            .enumerate()
            .find(|(_i, pane)| pane.widget.id() == id)
        {
            self.selected_pane_index = index;
        }
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
            .for_each(|(i, w)| w.widget.render(f, i == selected_pane_index));
    }
}
