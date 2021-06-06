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
    widgets: Vec<WidgetData<'a>>,
    layout: Layout,
    focused_widget_index: usize,
    focusable_widgets: Vec<usize>,
}

impl<'a> Tab<'a> {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        widgets: Vec<WidgetData<'a>>,
        layout: Layout,
    ) -> Self {
        let focusable_widgets = widgets
            .iter()
            .enumerate()
            .filter(|&(_, w)| w.widget.focusable())
            .map(|(i, _)| i)
            .collect();

        Self {
            id: id.into(),
            title: title.into(),
            widgets,
            layout,
            focusable_widgets,
            focused_widget_index: 0,
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

    pub fn as_ref_widgets(&self) -> Vec<&Widget<'a>> {
        self.widgets.iter().map(|w| &w.widget).collect()
    }

    pub fn as_mut_widgets(&mut self) -> Vec<&mut Widget<'a>> {
        self.widgets
            .iter_mut()
            .map(|w: &mut WidgetData| &mut w.widget)
            .collect()
    }

    pub fn next_widget(&mut self) {
        if self.focusable_widgets.len() - 1 <= self.focused_widget_index {
            self.focused_widget_index = 0;
        } else {
            self.focused_widget_index += 1;
        }
    }

    pub fn prev_widget(&mut self) {
        if self.focused_widget_index == 0 {
            self.focused_widget_index = self.focusable_widgets.len() - 1;
        } else {
            self.focused_widget_index -= 1;
        }
    }

    pub fn focused_widget_id(&self) -> &str {
        self.focused_widget().id()
    }

    pub fn focused_widget_mut(&mut self) -> &mut Widget<'a> {
        &mut self.widgets[self.focused_widget_index].widget
    }

    pub fn focused_widget(&self) -> &Widget {
        &self.widgets[self.focused_widget_index].widget
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        let chunks = self.layout.split(chunk);
        self.widgets
            .iter_mut()
            .for_each(|w| w.widget.update_chunk(chunks[w.chunk_index]));
    }

    pub fn focus_widget(&mut self, id: &str) {
        if let Some((index, _)) = self
            .widgets
            .iter()
            .enumerate()
            .find(|(_i, w)| w.widget.id() == id)
        {
            self.focused_widget_index = index;
        }
    }
}

impl Tab<'_> {
    pub fn render<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let focused_widget_index = self.focused_widget_index;

        self.widgets
            .iter_mut()
            .enumerate()
            .for_each(|(i, w)| w.widget.render(f, i == focused_widget_index));
    }
}
