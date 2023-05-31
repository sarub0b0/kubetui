use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Spans,
    widgets::{Block, Clear, Paragraph, Tabs},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::{
    event::{kubernetes::Kube, UserEvent},
    logger,
};

use super::{
    event::{EventResult, InnerCallback},
    util::{child_window_chunk, key_event_to_code, MousePosition, RectContainsPoint},
    widget::{RenderTrait, Widget, WidgetTrait},
    Tab,
};

type HeaderCallback = Rc<dyn Fn() -> Paragraph<'static>>;

#[derive(Default)]
pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    focused_tab_index: usize,
    layout: Layout,
    chunk: Rect,
    callbacks: Vec<(UserEvent, InnerCallback)>,
    popups: Vec<Widget<'a>>,
    open_popup_id: Option<String>,
    header: Option<Header<'a>>,
    layout_index: WindowLayoutIndex,
}

#[derive(Default)]
struct WindowLayoutIndex {
    tab: usize,
    header: usize,
    contents: usize,
}

pub enum HeaderContent<'a> {
    Static(Vec<Spans<'a>>),
    Callback(HeaderCallback),
}

impl Default for HeaderContent<'_> {
    fn default() -> Self {
        HeaderContent::Static(Default::default())
    }
}

#[derive(Default)]
pub struct Header<'a> {
    height: u16,
    content: HeaderContent<'a>,
}

impl<'a> Header<'a> {
    pub fn new_static(height: u16, content: Vec<Spans<'a>>) -> Self {
        debug_assert!(0 < height, "Header height must be greater than 0");

        Self {
            height,
            content: HeaderContent::Static(content),
        }
    }

    pub fn new_callback<F>(height: u16, callback: F) -> Self
    where
        F: Fn() -> Paragraph<'static> + 'static,
    {
        debug_assert!(0 < height, "Header height must be greater than 0");

        Self {
            height,
            content: HeaderContent::Callback(Rc::new(callback)),
        }
    }

    pub fn content_update(&mut self, content: HeaderContent<'a>) {
        self.content = content;
    }
}

#[derive(Default)]
pub struct WindowBuilder<'a> {
    tabs: Vec<Tab<'a>>,
    callbacks: Vec<(UserEvent, InnerCallback)>,
    popups: Vec<Widget<'a>>,
    header: Option<Header<'a>>,
}

impl<'a> WindowBuilder<'a> {
    pub fn tabs(mut self, tabs: impl Into<Vec<Tab<'a>>>) -> Self {
        self.tabs = tabs.into();
        self
    }

    pub fn action<F, E: Into<UserEvent>>(mut self, ev: E, cb: F) -> Self
    where
        F: Fn(&mut Window) -> EventResult + 'static,
    {
        self.callbacks.push((ev.into(), Rc::new(cb)));
        self
    }

    pub fn popup(mut self, popup: impl Into<Vec<Widget<'a>>>) -> Self {
        self.popups = popup.into();
        self
    }

    pub fn header(mut self, header: Header<'a>) -> Self {
        self.header = Some(header);
        self
    }

    pub fn build(self) -> Window<'a> {
        let (layout_index, constraints) = if let Some(header) = &self.header {
            (
                WindowLayoutIndex {
                    tab: 0,
                    header: 2,
                    contents: 3,
                },
                vec![
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(header.height),
                    Constraint::Min(1),
                ],
            )
        } else {
            (
                WindowLayoutIndex {
                    tab: 0,
                    header: 0,
                    contents: 2,
                },
                vec![
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ],
            )
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints);

        Window {
            tabs: self.tabs,
            layout,
            callbacks: self.callbacks,
            popups: self.popups,
            header: self.header,
            layout_index,
            ..Default::default()
        }
    }
}

// Window
impl<'a> Window<'a> {
    pub fn builder() -> WindowBuilder<'a> {
        WindowBuilder::default()
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let chunks = self.layout.split(chunk);

        let contents_index = self.layout_index.contents;
        self.tabs.iter_mut().for_each(|tab| {
            tab.update_chunk(chunks[contents_index]);
        });

        self.popups.iter_mut().for_each(|w| {
            let chunk = w
                .widget_config()
                .block()
                .inner(child_window_chunk(80, 80, chunk));

            w.update_chunk(chunk)
        })
    }

    fn chunks(&self) -> Rc<[Rect]> {
        self.layout.split(self.chunk)
    }

    pub fn widget(&self) -> Tabs {
        let titles: Vec<Spans> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| Spans::from(Self::tab_title_format(i, t.title())))
            .collect();

        Tabs::new(titles)
            .block(Self::tab_block())
            .select(self.focused_tab_index)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    }

    pub fn match_callback(&self, ev: UserEvent) -> Option<InnerCallback> {
        self.callbacks.iter().find_map(|(cb_ev, cb)| {
            logger!(debug, "match_callback {:?} <=> {:?}", ev, cb_ev);

            if *cb_ev == ev {
                Some(cb.clone())
            } else {
                None
            }
        })
    }

    pub fn update_header(&mut self, content: HeaderContent<'a>) {
        if let Some(h) = self.header.as_mut() {
            h.content_update(content);
        }
    }
}

// Popup
impl<'a> Window<'a> {
    pub fn open_popup(&mut self, id: impl Into<String>) {
        self.open_popup_id = Some(id.into());
    }

    pub fn close_popup(&mut self) {
        self.open_popup_id = None;
    }

    pub fn opening_popup(&self) -> bool {
        self.open_popup_id.is_some()
    }
}

// Tab
impl<'a> Window<'a> {
    pub fn focused_tab_id(&self) -> &str {
        self.tabs[self.focused_tab_index].id()
    }

    pub fn focused_tab(&self) -> &Tab<'a> {
        &self.tabs[self.focused_tab_index]
    }

    pub fn focused_tab_mut(&mut self) -> &mut Tab<'a> {
        &mut self.tabs[self.focused_tab_index]
    }

    pub fn focus_tab(&mut self, index: usize) {
        let index = index - 1;
        if index < self.tabs.len() {
            self.focused_tab_index = index;
        }
    }

    pub fn focus_next_tab(&mut self) {
        if self.tabs.len() - 1 <= self.focused_tab_index {
            self.focused_tab_index = 0;
        } else {
            self.focused_tab_index += 1;
        }
    }

    pub fn focus_prev_tab(&mut self) {
        if 0 == self.focused_tab_index {
            self.focused_tab_index = self.tabs.len() - 1;
        } else {
            self.focused_tab_index -= 1;
        }
    }

    fn tab_title_format(index: usize, title: &str) -> String {
        format!("{}: {}", index + 1, title)
    }

    fn tab_block() -> Block<'a> {
        Block::default().style(Style::default())
    }

    pub fn tab_chunk(&self) -> Rect {
        self.chunks()[self.layout_index.tab]
    }
}

// Pane
impl<'a> Window<'a> {
    pub fn find_widget(&self, id: &str) -> &Widget<'a> {
        if let Some(w) = self.popups.iter().find(|w| w.id() == id) {
            w
        } else {
            self.tabs
                .iter()
                .find_map(|t| t.find_widget(id))
                .unwrap_or_else(|| panic!("Could not find widget id [{}]", id))
        }
    }

    pub fn find_widget_mut(&mut self, id: &str) -> &mut Widget<'a> {
        if let Some(w) = self.popups.iter_mut().find(|w| w.id() == id) {
            w
        } else {
            self.tabs
                .iter_mut()
                .find_map(|t| t.find_widget_mut(id))
                .unwrap_or_else(|| panic!("Could not find widget id [{}]", id))
        }
    }

    pub fn focused_widget_id(&self) -> &str {
        self.focused_tab().focused_widget_id()
    }

    fn focus_next_widget(&mut self) {
        self.focused_tab_mut().next_widget();
    }

    fn focus_prev_widget(&mut self) {
        self.focused_tab_mut().prev_widget();
    }

    pub fn widget_clear(&mut self, id: &str) {
        self.find_widget_mut(id).clear();
    }

    pub fn focus_widget(&mut self, id: &str) {
        self.focused_tab_mut().focus_widget(id)
    }
}

// Render
impl<'a> Window<'a> {
    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        self.render_tab(f);

        self.render_header(f);

        self.render_contents(f);

        self.render_popup(f);
    }

    fn render_tab<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(self.widget(), self.tab_chunk());
    }

    fn render_header<B: Backend>(&self, f: &mut Frame<B>) {
        if let Some(header) = &self.header {
            let w = match &header.content {
                HeaderContent::Static(content) => Paragraph::new(content.to_vec()),
                HeaderContent::Callback(callback) => (callback)(),
            };
            f.render_widget(w, self.chunks()[self.layout_index.header]);
        }
    }

    fn render_contents<B: Backend>(&mut self, f: &mut Frame<B>) {
        self.focused_tab_mut().render(f);
    }

    fn render_popup<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let Some(id) = &self.open_popup_id {
            if let Some(popup) = self.popups.iter_mut().find(|p| p.id() == id) {
                f.render_widget(Clear, child_window_chunk(80, 80, self.chunk));
                popup.render(f, true);
            }
        }
    }
}

pub enum WindowEvent {
    CloseWindow,
    Continue,
    UpdateContents(Kube),
    ResizeWindow(u16, u16),
}

// Event
impl Window<'_> {
    pub fn on_event(&mut self, ev: UserEvent) -> EventResult {
        match ev {
            UserEvent::Key(ev) => self.on_key_event(ev),
            UserEvent::Mouse(ev) => self.on_mouse_event(ev),
            UserEvent::Resize(w, h) => EventResult::Window(WindowEvent::ResizeWindow(w, h)),
        }
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        if let Some(id) = &self.open_popup_id {
            if let Some(popup) = self.popups.iter_mut().find(|w| w.id() == id) {
                return popup.on_key_event(ev);
            }
        }

        let focus_widget = self.focused_tab_mut().focused_widget_mut();

        match focus_widget.on_key_event(ev) {
            EventResult::Ignore => match key_event_to_code(ev) {
                KeyCode::Tab => {
                    self.focus_next_widget();
                }

                KeyCode::BackTab => {
                    self.focus_prev_widget();
                }

                KeyCode::Char(n @ '1'..='9') => {
                    self.focus_tab(n as usize - b'0' as usize);
                }

                _ => {
                    return EventResult::Ignore;
                }
            },
            ev => {
                return ev;
            }
        }

        EventResult::Nop
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        if let Some(id) = &self.open_popup_id {
            if let Some(popup) = self.popups.iter_mut().find(|w| w.id() == id) {
                return popup.on_mouse_event(ev);
            }
        }

        let pos = (ev.column, ev.row);
        let focused_view_id = self.focused_widget_id().to_string();
        let mut focus_widget_id = None;

        let result = if self.tab_chunk().contains_point(pos) {
            self.on_click_tab(ev);
            EventResult::Nop
        } else if self.chunks()[self.layout_index.contents].contains_point(pos) {
            if let Some(w) = self
                .focused_tab_mut()
                .as_mut_widgets()
                .iter_mut()
                .find(|w| w.chunk().contains_point(pos))
            {
                focus_widget_id = if w.id() != focused_view_id {
                    Some(w.id().to_string())
                } else {
                    None
                };
                w.on_mouse_event(ev)
            } else {
                EventResult::Ignore
            }
        } else {
            EventResult::Ignore
        };

        if let Some(id) = focus_widget_id {
            self.focus_widget(&id);
        }

        result
    }

    fn on_click_tab(&mut self, ev: MouseEvent) {
        if ev.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }

        let pos = ev.position();

        let chunk = Self::tab_block().inner(self.tab_chunk());
        let divider_width = 1;

        let mut x = chunk.left();
        let y = chunk.top();
        let h = chunk.height;

        for (i, tab) in self.tabs.iter().enumerate() {
            let w = Self::tab_title_format(i, tab.title()).width() as u16;
            x = x.saturating_add(1);

            let title_chunk = Rect::new(x, y, w, h);

            if title_chunk.contains_point(pos) {
                self.focus_tab(i + 1);
                break;
            }

            x = x
                .saturating_add(1)
                .saturating_add(w)
                .saturating_add(divider_width);
        }
    }
}
