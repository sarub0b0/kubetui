use super::{
    event::EventResult,
    util::{MousePosition, RectContainsPoint},
    widget::*,
};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    prelude::Direction,
    Frame,
};

use std::rc::Rc;

pub enum LayoutElement {
    WidgetIndex(usize),
    NestedElement(NestedWidgetLayout),
}

pub struct NestedLayoutElement(pub Constraint, pub LayoutElement);

pub struct NestedWidgetLayout {
    layout: Layout,
    elements: Vec<LayoutElement>,
}

impl Default for NestedWidgetLayout {
    fn default() -> Self {
        Self {
            layout: Layout::default().constraints([Constraint::Percentage(100)]),
            elements: Default::default(),
        }
    }
}

impl NestedWidgetLayout {
    pub fn direction(mut self, direction: Direction) -> Self {
        self.layout = self.layout.direction(direction);
        self
    }

    pub fn nested_widget_layout(
        mut self,
        nested_layout_elements: impl Into<Vec<NestedLayoutElement>>,
    ) -> Self {
        let configs: Vec<_> = nested_layout_elements.into();

        let (constraints, elements): (Vec<_>, Vec<_>) = configs
            .into_iter()
            .map(|NestedLayoutElement(constraint, element)| (constraint, element))
            .unzip();

        self.layout = self.layout.constraints(constraints);
        self.elements = elements;

        self
    }

    fn split(&self, chunk: Rect) -> Rc<[Rect]> {
        self.layout.split(chunk)
    }

    fn update_chunk(&mut self, chunk: Rect, widgets: &mut [Widget<'_>]) {
        let chunks = self.layout.split(chunk);

        chunks
            .iter()
            .zip(self.elements.iter_mut())
            .for_each(|(chunk, layout_element)| match layout_element {
                LayoutElement::WidgetIndex(i) => widgets[*i].update_chunk(*chunk),
                LayoutElement::NestedElement(element) => element.update_chunk(*chunk, widgets),
            });
    }
}

pub struct Tab<'a> {
    id: String,
    title: String,
    nested_widget_layout: NestedWidgetLayout,
    widgets: Vec<Widget<'a>>,
    active_widget_index: usize,
    activatable_widget_indices: Vec<usize>,
    mouse_over_widget_index: Option<usize>,
}

#[allow(dead_code)]
impl<'a> Tab<'a> {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        widgets: impl Into<Vec<Widget<'a>>>,
        layout: NestedWidgetLayout,
    ) -> Self {
        let widgets: Vec<_> = widgets.into();

        let activatable_widget_indices = widgets
            .iter()
            .enumerate()
            .filter(|(_, w)| w.can_activate())
            .map(|(i, _)| i)
            .collect();

        Self {
            id: id.into(),
            title: title.into(),
            nested_widget_layout: layout,
            widgets,
            activatable_widget_indices,
            active_widget_index: 0,
            mouse_over_widget_index: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn chunks(&self, tab_size: Rect) -> Rc<[Rect]> {
        self.nested_widget_layout.split(tab_size)
    }

    pub fn as_ref_widgets(&self) -> Vec<&Widget<'a>> {
        self.widgets.iter().collect()
    }

    pub fn as_mut_widgets(&mut self) -> Vec<&mut Widget<'a>> {
        self.widgets.iter_mut().collect()
    }

    pub fn activate_next_widget(&mut self) {
        self.clear_mouse_over();

        self.active_widget_index =
            (self.active_widget_index + 1) % self.activatable_widget_indices.len();
    }

    pub fn activate_prev_widget(&mut self) {
        self.clear_mouse_over();

        let activatable_widget_len = self.activatable_widget_indices.len();

        self.active_widget_index =
            (self.active_widget_index + activatable_widget_len - 1) % activatable_widget_len;
    }

    pub fn active_widget_id(&self) -> &str {
        self.active_widget().id()
    }

    pub fn active_widget_mut(&mut self) -> &mut Widget<'a> {
        &mut self.widgets[self.active_widget_index]
    }

    pub fn active_widget(&self) -> &Widget<'a> {
        &self.widgets[self.active_widget_index]
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.nested_widget_layout
            .update_chunk(chunk, &mut self.widgets);
    }

    pub fn activate_widget_by_id(&mut self, id: &str) {
        if let Some((index, _)) = self.widgets.iter().enumerate().find(|(_, w)| w.id() == id) {
            self.clear_mouse_over();

            self.active_widget_index = index;
        }
    }

    pub fn clear_mouse_over(&mut self) {
        self.mouse_over_widget_index = None;
    }

    pub fn find_widget(&self, id: &str) -> Option<&Widget<'a>> {
        self.widgets.iter().find(|w| w.id() == id)
    }

    pub fn find_widget_mut(&mut self, id: &str) -> Option<&mut Widget<'a>> {
        self.widgets.iter_mut().find(|w| w.id() == id)
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = ev.position();

        let active_widget_id = self.active_widget_id().to_string();

        let Some((index, id)) = self
            .as_mut_widgets()
            .iter_mut()
            .enumerate()
            .find(|(_, w)| w.chunk().contains_point(pos))
            .map(|(i, w)| (i, w.id().to_string()))
        else {
            return EventResult::Ignore;
        };

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if id != active_widget_id {
                    self.activate_widget_by_id(&id);
                }
            }
            MouseEventKind::Moved => {
                self.mouse_over_widget_index = Some(index);
            }
            _ => {}
        }

        self.active_widget_mut().on_mouse_event(ev)
    }
}

impl<'a> Tab<'a> {
    pub fn render(&mut self, f: &mut Frame) {
        self.widgets.iter_mut().enumerate().for_each(|(i, w)| {
            w.render(
                f,
                i == self.active_widget_index,
                self.mouse_over_widget_index.is_some_and(|idx| idx == i),
            )
        });
    }
}
