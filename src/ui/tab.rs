use super::{
    event::EventResult,
    util::{MousePosition, RectContainsPoint},
    widget::*,
};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    Frame,
};

use std::rc::Rc;

pub struct WidgetChunk<'a> {
    chunk_index: usize,
    widget: Widget<'a>,
}

impl<'a> WidgetChunk<'a> {
    pub fn new(widget: impl Into<Widget<'a>>) -> Self {
        Self {
            widget: widget.into(),
            chunk_index: 0,
        }
    }

    pub fn chunk_index(mut self, index: usize) -> Self {
        self.chunk_index = index;
        self
    }
}

pub struct Tab<'a> {
    id: String,
    title: String,
    widgets: Vec<WidgetChunk<'a>>,
    layout: Layout,
    active_widget_index: usize,
    activatable_widget_indices: Vec<usize>,
    mouse_over_widget_index: Option<usize>,
}

impl<'a> Tab<'a> {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        widgets: impl Into<Vec<WidgetChunk<'a>>>,
    ) -> Self {
        let widgets = widgets.into();
        let activatable_widget_indices = widgets
            .iter()
            .enumerate()
            .filter(|&(_, w)| w.widget.can_activate())
            .map(|(i, _)| i)
            .collect();

        let layout = Layout::default().constraints([Constraint::Percentage(100)]);

        Self {
            id: id.into(),
            title: title.into(),
            widgets,
            layout,
            activatable_widget_indices,
            active_widget_index: 0,
            mouse_over_widget_index: None,
        }
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn chunks(&self, tab_size: Rect) -> Rc<[Rect]> {
        self.layout.split(tab_size)
    }

    pub fn as_ref_widgets(&self) -> Vec<&Widget<'a>> {
        self.widgets.iter().map(|w| &w.widget).collect()
    }

    pub fn as_mut_widgets(&mut self) -> Vec<&mut Widget<'a>> {
        self.widgets
            .iter_mut()
            .map(|w: &mut WidgetChunk| &mut w.widget)
            .collect()
    }

    pub fn activate_next_widget(&mut self) {
        self.mouse_over_widget_index = None;

        self.active_widget_index =
            (self.active_widget_index + 1) % self.activatable_widget_indices.len();
    }

    pub fn activate_prev_widget(&mut self) {
        self.mouse_over_widget_index = None;

        let activatable_widget_len = self.activatable_widget_indices.len();

        self.active_widget_index =
            (self.active_widget_index + activatable_widget_len - 1) % activatable_widget_len;
    }

    pub fn active_widget_id(&self) -> &str {
        self.active_widget().id()
    }

    pub fn active_widget_mut(&mut self) -> &mut Widget<'a> {
        &mut self.widgets[self.active_widget_index].widget
    }

    pub fn active_widget(&self) -> &Widget<'a> {
        &self.widgets[self.active_widget_index].widget
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        let chunks = self.layout.split(chunk);
        self.widgets
            .iter_mut()
            .for_each(|w| w.widget.update_chunk(chunks[w.chunk_index]));
    }

    pub fn activate_widget_by_id(&mut self, id: &str) {
        if let Some((index, _)) = self
            .widgets
            .iter()
            .enumerate()
            .find(|(_, w)| w.widget.id() == id)
        {
            self.mouse_over_widget_index = None;

            self.active_widget_index = index;
        }
    }

    pub fn find_widget(&self, id: &str) -> Option<&Widget<'a>> {
        self.widgets.iter().find_map(|w| {
            if w.widget.id() == id {
                Some(&w.widget)
            } else {
                None
            }
        })
    }

    pub fn find_widget_mut(&mut self, id: &str) -> Option<&mut Widget<'a>> {
        self.widgets.iter_mut().find_map(|w| {
            if w.widget.id() == id {
                Some(&mut w.widget)
            } else {
                None
            }
        })
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = ev.position();

        let active_widget_id = self.active_widget_id().to_string();

        let Some((index, id)) = self
            .as_mut_widgets()
            .iter_mut()
            .enumerate()
            .find(|(_, w)| w.chunk().contains_point(pos))
            .map(|(i, w)| (i, w.id().to_string()) ) else { return EventResult::Ignore };

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if id != active_widget_id {
                    self.activate_widget_by_id(&id);

                    return EventResult::Ignore;
                }
            }
            _ => {
                self.mouse_over_widget_index = Some(index);
            }
        }

        self.active_widget_mut().on_mouse_event(ev)
    }
}

impl Tab<'_> {
    pub fn render<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        self.widgets.iter_mut().enumerate().for_each(|(i, w)| {
            w.widget.render(
                f,
                i == self.active_widget_index,
                self.mouse_over_widget_index.is_some_and(|idx| idx == i),
            )
        });
    }
}
