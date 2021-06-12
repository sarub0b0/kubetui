use chrono::Local;
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Clear, Paragraph, Tabs},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::util::child_window_chunk;

use super::{
    event::{EventResult, InnerCallback},
    util,
    widget::{RenderTrait, Widget, WidgetTrait},
    Tab,
};

use ::event::UserEvent;

#[derive(Default)]
pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    focused_tab_index: usize,
    layout: Layout,
    chunk: Rect,
    status_target_id: Vec<(&'a str, &'a str)>,
    callbacks: Vec<(UserEvent, InnerCallback)>,
    popup: Vec<Widget<'a>>,
    open_popup_id: Option<String>,
}

pub mod window_layout_index {
    pub const TAB: usize = 0;
    pub const CONTEXT: usize = 2;
    pub const CONTENTS: usize = 3;
    pub const STATUSBAR: usize = 4;
}

// Window
impl<'a> Window<'a> {
    pub fn new(tabs: impl Into<Vec<Tab<'a>>>) -> Self {
        Self {
            tabs: tabs.into(),
            focused_tab_index: 0,
            layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(2),
                        Constraint::Min(1),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                ),
            ..Default::default()
        }
    }

    pub fn add_popup(&mut self, popup: impl Into<Vec<Widget<'a>>>) {
        self.popup = popup.into();
    }

    pub fn status_target_id(mut self, id: impl Into<Vec<(&'a str, &'a str)>>) -> Self {
        self.status_target_id = id.into();
        self
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let chunks = self.layout.split(chunk);

        self.tabs.iter_mut().for_each(|tab| {
            tab.update_chunk(chunks[window_layout_index::CONTENTS]);
        });

        self.popup.iter_mut().for_each(|w| {
            w.update_chunk(util::default_focus_block().inner(child_window_chunk(80, 80, chunk)))
        })
    }

    fn chunks(&self) -> Vec<Rect> {
        self.layout.split(self.chunk)
    }

    pub fn widget(&self) -> Tabs {
        let titles: Vec<Spans> = self
            .tabs
            .iter()
            .map(|t| Spans::from(Self::tab_title_format(t.title())))
            .collect();

        Tabs::new(titles)
            .block(Self::tab_block())
            .select(self.focused_tab_index)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    }

    pub fn match_callback(&self, ev: UserEvent) -> Option<InnerCallback> {
        self.callbacks
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb.clone()) } else { None })
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

    pub fn popup(&self, id: &str) -> Option<&Widget<'a>> {
        self.popup.iter().find(|w| w.id() == id)
    }

    pub fn popup_mut(&mut self, id: &str) -> Option<&mut Widget<'a>> {
        self.popup.iter_mut().find(|w| w.id() == id)
    }
}

// Tab
impl<'a> Window<'a> {
    pub fn focused_tab_id(&self) -> &str {
        &self.tabs[self.focused_tab_index].id()
    }

    pub fn focused_tab(&self) -> &Tab {
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

    fn tab_title_format(title: &str) -> String {
        format!(" {} ", title)
    }

    fn tab_block() -> Block<'a> {
        Block::default().style(Style::default())
    }

    pub fn tab_chunk(&self) -> Rect {
        self.chunks()[window_layout_index::TAB]
    }
}

// Pane
impl<'a> Window<'a> {
    pub fn find_widget(&self, id: &str) -> Option<&Widget<'a>> {
        for t in &self.tabs {
            let w = t.as_ref_widgets().into_iter().find(|w| w.id() == id);
            if w.is_some() {
                return w;
            }
        }

        self.popup(id)
    }
    pub fn find_widget_mut(&mut self, id: &str) -> Option<&mut Widget<'a>> {
        if self.opening_popup() {
            let w = self.popup.iter_mut().find(|w| w.id() == id);

            if w.is_some() {
                w
            } else {
                self.tabs
                    .iter_mut()
                    .find_map(|t| t.as_mut_widgets().into_iter().find(|w| w.id() == id))
            }
        } else {
            let w = self
                .tabs
                .iter_mut()
                .find_map(|t| t.as_mut_widgets().into_iter().find(|w| w.id() == id));

            if w.is_some() {
                w
            } else {
                self.popup.iter_mut().find(|w| w.id() == id)
            }
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
        if let Some(w) = self.find_widget_mut(id) {
            w.clear();
        }
    }

    pub fn focus_widget(&mut self, id: &str) {
        self.focused_tab_mut().focus_widget(id)
    }
}

// Render
use window_layout_index::*;
impl<'a> Window<'a> {
    pub fn render<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        current_context: &str,
        current_namespaces: &[String],
    ) {
        self.render_tab(f);

        self.render_context(f, current_context, current_namespaces);

        self.focused_tab_mut().render(f);

        self.render_status(f);

        if let Some(id) = &self.open_popup_id {
            if let Some(popup) = self.popup.iter_mut().find(|p| p.id() == id) {
                f.render_widget(Clear, child_window_chunk(80, 80, self.chunk));
                popup.render(f, true);
            }
        }
    }

    fn render_tab<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(self.widget(), self.tab_chunk());
    }

    fn render_context<B: Backend>(&mut self, f: &mut Frame<B>, ctx: &str, ns: &[String]) {
        let block = Block::default().style(Style::default());

        let spans_ctx = Spans::from(format!("ctx: {}", ctx));
        let mut spans_ns = vec![Span::raw("ns: ")];
        spans_ns.extend(
            ns.iter()
                .enumerate()
                .map(|(i, ns)| {
                    let mut s = String::default();
                    if i != 0 {
                        s += ", ";
                    }

                    s += &format!("{}", ns);

                    Span::raw(s)
                })
                .collect::<Vec<Span>>(),
        );
        let paragraph = Paragraph::new(vec![spans_ctx, Spans::from(spans_ns)]).block(block);

        f.render_widget(paragraph, self.chunks()[CONTEXT]);
    }

    fn render_status<B: Backend>(&mut self, f: &mut Frame<B>) {
        let scroll_status_spans = self.scroll_status();
        let datetime_spans = Spans::from(datetime());

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(datetime_spans.width() as u16),
                Constraint::Length(scroll_status_spans.width() as u16),
            ])
            .split(self.chunks()[STATUSBAR]);

        f.render_widget(
            Paragraph::new(datetime_spans).block(Block::default().style(Style::default())),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(scroll_status_spans)
                .block(Block::default().style(Style::default()))
                .alignment(Alignment::Right),
            chunks[1],
        );
    }

    fn scroll_status(&self) -> Spans {
        if let Some(id) = self
            .status_target_id
            .iter()
            .find(|id| id.0 == self.focused_tab_id())
        {
            if let Some(w) = self
                .focused_tab()
                .as_ref_widgets()
                .iter()
                .find(|w| w.id() == id.1)
            {
                return w.as_text().status();
            }
        }

        Spans::default()
    }
}

fn datetime() -> Span<'static> {
    Span::raw(format!(
        " {}",
        Local::now().format("%Y年%m月%d日 %H時%M分%S秒")
    ))
}

pub enum WindowEvent {
    CloseWindow,
    Continue,
    UpdateContents(::event::kubernetes::Kube),
    ResizeWindow(u16, u16),
}

// Event
impl Window<'_> {
    pub fn add_action<F, E: Into<UserEvent>>(&mut self, ev: E, cb: F)
    where
        F: Fn(&mut Window) -> EventResult + 'static,
    {
        self.callbacks.push((ev.into(), Rc::new(cb)));
    }

    pub fn on_event(&mut self, ev: UserEvent) -> EventResult {
        match ev {
            UserEvent::Key(ev) => self.on_key_event(ev),
            UserEvent::Mouse(ev) => self.on_mouse_event(ev),
            UserEvent::Resize(w, h) => EventResult::Window(WindowEvent::ResizeWindow(w, h)),
        }
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        if let Some(id) = &self.open_popup_id {
            if let Some(popup) = self.popup.iter_mut().find(|w| w.id() == id) {
                return popup.on_key_event(ev);
            }
        }

        let focus_widget = self.focused_tab_mut().focused_widget_mut();

        match focus_widget.on_key_event(ev) {
            EventResult::Ignore => match util::key_event_to_code(ev) {
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
            if let Some(popup) = self.popup.iter_mut().find(|w| w.id() == id) {
                return popup.on_mouse_event(ev);
            }
        }

        let pos = (ev.column, ev.row);
        let focused_view_id = self.focused_widget_id().to_string();
        let mut focus_widget_id = None;

        let result = if util::contains(self.tab_chunk(), pos) {
            self.on_click_tab(ev);
            EventResult::Nop
        } else if util::contains(self.chunks()[window_layout_index::CONTENTS], pos) {
            if let Some(w) = self
                .focused_tab_mut()
                .as_mut_widgets()
                .iter_mut()
                .find(|w| util::contains(w.chunk(), pos))
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

        let pos = util::mouse_pos(ev);

        let chunk = Self::tab_block().inner(self.tab_chunk());
        let divider_width = 1;

        let mut x = chunk.left();
        let y = chunk.top();
        let h = chunk.height;

        for (i, tab) in self.tabs.iter().enumerate() {
            let w = Self::tab_title_format(tab.title()).width() as u16;
            x = x.saturating_add(1);

            let title_chunk = Rect::new(x, y, w, h);

            if util::contains(title_chunk, pos) {
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
