use crate::Window;
use crossterm::event::KeyEvent;
use event::UserEvent;
use std::rc::Rc;

use crate::widget::WidgetItem;
use crate::EventResult;

use super::crossterm::event::MouseEvent;
use super::tui::{
    backend::Backend,
    layout::Rect,
    text::Span,
    widgets::{Block, Clear},
    Frame,
};

use super::{child_window_chunk, focus_title_style, Pane};

use super::complex_widgets::{MultipleSelect, SingleSelect};

pub type InnerCallback = Rc<dyn Fn(&mut Window) -> EventResult>;

pub enum SubWidget<'a> {
    Pane(Pane<'a>),
    Multiple(MultipleSelect<'a>),
    Single(SingleSelect<'a>),
}

impl<'a> SubWidget<'a> {
    fn sub_widget(&self) -> &Self {
        self
    }

    fn sub_widget_mut(&mut self) -> &mut Self {
        self
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        match self {
            SubWidget::Pane(w) => w.on_mouse_event(ev),
            SubWidget::Multiple(w) => w.on_mouse_event(ev),
            SubWidget::Single(w) => w.on_mouse_event(ev),
        }
    }

    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        match self {
            SubWidget::Pane(w) => w.render(f, true),
            SubWidget::Multiple(w) => w.render(f),
            SubWidget::Single(w) => w.render(f),
        }
    }

    fn update_chunks(&mut self, chunk: Rect) {
        match self {
            SubWidget::Pane(w) => w.update_chunk(chunk),
            SubWidget::Multiple(w) => w.update_chunk(chunk),
            SubWidget::Single(w) => w.update_chunk(chunk),
        }
    }

    pub fn set_items(&mut self, items: WidgetItem) {
        match self {
            SubWidget::Pane(w) => w.set_items(items),
            SubWidget::Multiple(w) => w.set_items(items.array()),
            SubWidget::Single(w) => w.set_items(items.array()),
        }
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match self {
            SubWidget::Pane(w) => w.on_key_event(ev),
            SubWidget::Multiple(w) => w.on_key_event(ev),
            SubWidget::Single(w) => w.on_key_event(ev),
        }
    }

    pub fn match_callback(&self, ev: UserEvent) -> Option<InnerCallback> {
        match self {
            SubWidget::Pane(_w) => None,
            SubWidget::Multiple(_w) => None,
            SubWidget::Single(w) => w.match_callback(ev),
        }
    }
}

// pub trait PaneTrait {
//     fn id(&self) -> &str;
//     fn update_chunks(&mut self, chunk: Rect);
//     fn select_next_pane(&mut self) {}
//     fn select_prev_pane(&mut self) {}
//     fn select_next_item(&mut self) {}
//     fn select_prev_item(&mut self) {}
//     fn select_first_item(&mut self) {}
//     fn select_last_item(&mut self) {}
//     fn set_items(&mut self, id: &str, items: WidgetItem);
//     fn get_item(&self, _: &str) -> Option<WidgetItem> {
//         None
//     }
//     fn render<B: Backend>(&mut self, f: &mut Frame<B>);
//     fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult;
// }

pub struct SubWindow<'a> {
    id: String,
    title: String,
    chunk: Rect,
    pane: SubWidget<'a>,
    block: Option<Block<'a>>,
}

impl<'a> SubWindow<'a> {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        pane: SubWidget<'a>,
        block: Option<Block<'a>>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            chunk: Rect::default(),
            pane,
            block,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = child_window_chunk(80, 80, chunk);

        let inner_chunk = match self.block {
            Some(ref b) => b.inner(b.inner(self.chunk)),
            None => self.chunk,
        };

        self.pane.update_chunks(inner_chunk);
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(Clear, self.chunk);

        if let Some(ref b) = self.block {
            f.render_widget(
                b.clone()
                    .title(Span::styled(self.title.as_str(), focus_title_style(true)))
                    .title_offset(1),
                b.inner(self.chunk),
            );
        }

        self.pane.render(f);
    }

    pub fn pane(&self) -> &SubWidget {
        self.pane.sub_widget()
    }

    pub fn pane_mut(&mut self) -> &mut SubWidget<'a> {
        self.pane.sub_widget_mut()
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        self.pane.on_mouse_event(ev)
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.pane.on_key_event(ev)
    }

    pub fn on_event(&mut self, ev: UserEvent) -> EventResult {
        match ev {
            UserEvent::Key(ev) => self.on_key_event(ev),
            UserEvent::Mouse(ev) => self.on_mouse_event(ev),
            UserEvent::Resize(_, _) => EventResult::Ignore,
        }
    }

    pub fn match_callback(&mut self, ev: UserEvent) -> Option<InnerCallback> {
        self.pane.match_callback(ev)
    }
}

// impl
// impl<'a> PaneTrait for Pane<'a> {
//     fn id(&self) -> &str {
//         self.id()
//     }

//     fn update_chunks(&mut self, chunk: Rect) {
//         self.update_chunk(chunk);
//     }

//     fn select_first_item(&mut self) {
//         self.select_first_item();
//     }

//     fn select_last_item(&mut self) {
//         self.select_last_item();
//     }

//     fn set_items(&mut self, _id: &str, items: WidgetItem) {
//         self.widget_mut().set_items(items);
//     }

//     fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
//         self.render(f, true);
//     }

//     fn get_item(&self, _id: &str) -> Option<WidgetItem> {
//         self.widget().get_item()
//     }

//     fn select_next_item(&mut self) {
//         self.select_next_item(1)
//     }

//     fn select_prev_item(&mut self) {
//         self.select_prev_item(1)
//     }

//     fn get(&self) -> &SubWidget {
//         self
//     }

//     fn get_mut(&mut self) -> &mut SubWidget {
//         self
//     }

//     fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
//         self.on_mouse_event(ev)
//     }
// }

// impl<'a> PaneTrait for MultipleSelect<'a> {
//     type Item = MultipleSelect<'a>;

//     fn update_chunks(&mut self, chunk: Rect) {
//         self.update_chunk(chunk)
//     }

//     fn select_next_pane(&mut self) {
//         self.toggle_focus()
//     }

//     fn select_prev_pane(&mut self) {
//         self.toggle_focus()
//     }

//     fn select_next_item(&mut self) {
//         self.select_next_item()
//     }

//     fn select_prev_item(&mut self) {
//         self.select_prev_item()
//     }

//     fn set_items(&mut self, _id: &str, items: WidgetItem) {
//         self.set_list_items(items.array())
//     }

//     fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
//         self.render(f);
//     }

//     fn get(&self) -> &SubWidget {
//         self
//     }

//     fn get_mut(&mut self) -> &mut SubWidget {
//         self
//     }

//     fn id(&self) -> &str {
//         self.id()
//     }

//     fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
//         self.on_mouse_event(ev)
//     }
// }

// impl<'a> PaneTrait for SingleSelect<'a> {
//     fn select_next_item(&mut self) {
//         self.select_next_item()
//     }

//     fn select_prev_item(&mut self) {
//         self.select_prev_item()
//     }

//     fn set_items(&mut self, _id: &str, items: WidgetItem) {
//         self.set_items(items.array())
//     }

//     fn get_item(&self, _id: &str) -> Option<WidgetItem> {
//         self.get_item()
//     }

//     fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
//         self.render(f)
//     }

//     fn get(&self) -> &SubWidget {
//         self
//     }

//     fn get_mut(&mut self) -> &mut SubWidget {
//         self
//     }

//     fn id(&self) -> &str {
//         self.id()
//     }

//     fn update_chunks(&mut self, chunk: Rect) {
//         self.update_chunk(chunk);
//     }
//     fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
//         self.on_mouse_event(ev)
//     }
// }
