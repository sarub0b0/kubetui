// use std::sync::mpsc::{self, Receiver, Sender};
use crossbeam::channel::{Receiver, Sender};

use chrono::Local;
use crossterm::event::{KeyCode, KeyModifiers};

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};

use event::{kubernetes::*, Event};
use tui_wrapper::{widget::*, *};

use component::select::*;
use sub_window::*;

pub mod view_id {

    #![allow(non_upper_case_globals)]
    macro_rules! generate_id {
        ($id:ident) => {
            pub const $id: &str = stringify!($id);
        };
    }

    generate_id!(tab_pods);
    generate_id!(tab_pods_pane_pods);
    generate_id!(tab_pods_pane_logs);
    generate_id!(tab_configs);
    generate_id!(tab_configs_pane_configs);
    generate_id!(tab_configs_pane_raw_data);
    generate_id!(tab_event);
    generate_id!(tab_event_pane_event);
    generate_id!(tab_apis);
    generate_id!(tab_apis_pane_apis);

    generate_id!(subwin_ns);
    generate_id!(subwin_ns_pane_ns);
    generate_id!(subwin_apis);
    generate_id!(subwin_apis_pane);
    generate_id!(subwin_apis_pane_filter);
    generate_id!(subwin_apis_pane_items);
    generate_id!(subwin_apis_pane_selected);
}

pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    selected_tab_index: usize,
    layout: Layout,
    chunk: Rect,
}

pub mod window_layout_index {
    pub const TAB: usize = 0;
    pub const CONTEXT: usize = 2;
    pub const CONTENTS: usize = 3;
    pub const STATUSBAR: usize = 4;
}

// Window
impl<'a> Window<'a> {
    pub fn new(tabs: Vec<Tab<'a>>) -> Self {
        Self {
            tabs,
            selected_tab_index: 0,
            layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(1),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                ),
            chunk: Default::default(),
        }
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let chunks = self.layout.split(chunk);

        self.tabs.iter_mut().for_each(|tab| {
            tab.update_chunk(chunks[window_layout_index::CONTENTS]);
        });
    }

    pub fn chunks(&self) -> Vec<Rect> {
        self.layout.split(self.chunk)
    }

    pub fn widget(&self) -> Tabs {
        let titles: Vec<Spans> = self
            .tabs
            .iter()
            .map(|t| Spans::from(format!(" {} ", t.title())))
            .collect();

        let block = Block::default().style(Style::default());

        Tabs::new(titles)
            .block(block)
            .select(self.selected_tab_index)
            .highlight_style(Style::default().fg(Color::White).bg(Color::LightBlue))
    }

    pub fn tab_chunk(&self) -> Rect {
        let chunk = self.chunks()[window_layout_index::TAB];
        if chunk.height == 0 {
            Rect::new(chunk.x, chunk.y, chunk.width, 1)
        } else {
            chunk
        }
    }
}

// Tab
impl<'a> Window<'a> {
    pub fn selected_tab_id(&self) -> &str {
        &self.tabs[self.selected_tab_index].id()
    }

    pub fn selected_tab(&self) -> &Tab {
        &self.tabs[self.selected_tab_index]
    }

    pub fn selected_tab_mut(&mut self) -> &mut Tab<'a> {
        &mut self.tabs[self.selected_tab_index]
    }

    pub fn select_tab(&mut self, index: usize) {
        let index = index - 1;
        if index < self.tabs.len() {
            self.selected_tab_index = index;
        }
    }

    pub fn select_next_tab(&mut self) {
        if self.tabs.len() - 1 <= self.selected_tab_index {
            self.selected_tab_index = 0;
        } else {
            self.selected_tab_index += 1;
        }
    }

    pub fn select_prev_tab(&mut self) {
        if 0 == self.selected_tab_index {
            self.selected_tab_index = self.tabs.len() - 1;
        } else {
            self.selected_tab_index -= 1;
        }
    }
}

// Pane
impl<'a> Window<'a> {
    pub fn pane(&self, id: &str) -> Option<&Pane<'a>> {
        for t in &self.tabs {
            let p = t.widgets().iter().find(|p| p.id() == id);
            if p.is_some() {
                return p;
            }
        }
        None
    }
    pub fn pane_mut(&mut self, id: &str) -> Option<&mut Pane<'a>> {
        for t in &mut self.tabs {
            let p = t.widgets_mut().iter_mut().find(|p| p.id() == id);
            if p.is_some() {
                return p;
            }
        }
        None
    }

    pub fn selected_pane_id(&self) -> &str {
        self.selected_tab().selected_pane_id()
    }

    pub fn select_next_pane(&mut self) {
        self.selected_tab_mut().next_pane();
    }

    pub fn select_prev_pane(&mut self) {
        self.selected_tab_mut().prev_pane();
    }

    pub fn pane_clear(&mut self, id: &str) {
        if let Some(pane) = self.pane_mut(id) {
            pane.clear();
        }
    }

    pub fn select_pane(&mut self, id: &str) {
        self.selected_tab_mut().select_pane(id)
    }
}

// フォーカスしているwidgetの状態変更
impl Window<'_> {
    pub fn select_next_item(&mut self) {
        self.selected_tab_mut().select_pane_next_item();
    }

    pub fn select_prev_item(&mut self) {
        self.selected_tab_mut().select_pane_prev_item();
    }

    pub fn select_first_item(&mut self) {
        self.selected_tab_mut().select_pane_first_item();
    }

    pub fn select_last_item(&mut self) {
        self.selected_tab_mut().select_pane_last_item();
    }

    pub fn scroll_up(&mut self) {
        let pane = self.selected_tab_mut().selected_pane_mut();
        let ch = pane.chunk();

        match pane.widget_mut() {
            Widget::List(list) => {
                list.prev();
            }
            Widget::Text(text) => {
                text.scroll_up(ch.height as u64);
            }
            Widget::Table(table) => {
                table.prev();
            }
        }
    }

    pub fn scroll_down(&mut self) {
        let pane = self.selected_tab_mut().selected_pane_mut();
        let ch = pane.chunk();

        match pane.widget_mut() {
            Widget::List(list) => {
                list.next();
            }
            Widget::Text(text) => {
                text.scroll_down(ch.height as u64);
            }
            Widget::Table(table) => {
                table.next();
            }
        }
    }
}

// Render
use window_layout_index::*;
impl<'a> Window<'a> {
    pub fn render<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        current_context: &str,
        current_namespace: &str,
    ) {
        self.render_tab(f);

        self.render_context(f, current_context, current_namespace);

        self.selected_tab_mut().render(f);

        self.render_status(f);
    }

    fn render_tab<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(self.widget(), self.tab_chunk());
    }

    fn render_context<B: Backend>(&mut self, f: &mut Frame<B>, ctx: &str, ns: &str) {
        let block = Block::default().style(Style::default());

        let text = format!("{}: {}", ns, ctx);
        let spans = Spans::from(text);
        let paragraph = Paragraph::new(spans).block(block);

        f.render_widget(paragraph, self.chunks()[CONTEXT]);
    }
    fn render_status<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(self.chunks()[STATUSBAR]);

        let datetime = datetime();

        let datetime = Spans::from(datetime);
        let block = Block::default().style(Style::default());
        let paragraph = Paragraph::new(datetime).block(block);

        f.render_widget(paragraph, chunks[0]);

        let widget = match self.selected_tab_id() {
            view_id::tab_pods => self.scroll_status(view_id::tab_pods_pane_logs),
            view_id::tab_configs => self.scroll_status(view_id::tab_configs_pane_raw_data),
            view_id::tab_event => self.scroll_status(view_id::tab_event_pane_event),
            view_id::tab_apis => None,
            _ => unreachable!(),
        };

        if let Some(widget) = widget {
            f.render_widget(widget, chunks[1]);
        }
    }

    fn scroll_status(&self, id: &str) -> Option<Paragraph<'a>> {
        if let Some(pane) = self.selected_tab().widgets().iter().find(|p| p.id() == id) {
            let widget = pane.widget().text();
            let span = match widget {
                Some(t) => text_status((t.selected(), t.row_size())),
                None => text_status((0, 0)),
            };

            let spans = Spans::from(span);
            let block = Block::default().style(Style::default());

            return Some(
                Paragraph::new(spans)
                    .block(block)
                    .alignment(Alignment::Right),
            );
        }
        None
    }
}

fn datetime() -> Span<'static> {
    Span::raw(format!(
        " {}",
        Local::now().format("%Y年%m月%d日 %H時%M分%S秒")
    ))
}

fn text_status((current, rows): (u64, u64)) -> Span<'static> {
    Span::raw(format!("{}/{}", current, rows))
}

pub enum WindowEvent {
    CloseWindow,
    Continue,
    OpenSubWindow(&'static str),
    CloseSubWindow,
    ResizeWindow,
}

fn update_event(window: &mut Window, ev: Vec<String>) {
    let pane = window.pane_mut(view_id::tab_event_pane_event);
    if let Some(p) = pane {
        let widget = p.widget_mut().text_mut().unwrap();

        let old_select = widget.selected();
        let is_bottom = widget.is_bottom();

        widget.set_items(WidgetItem::Array(ev));

        let new_len = widget.spans().len();

        if is_bottom || (new_len < old_select as usize) {
            widget.select_last();
        } else {
            widget.select(old_select);
        }
    }
}

fn update_pod_logs(window: &mut Window, logs: Vec<String>) {
    let pane = window.pane_mut(view_id::tab_pods_pane_logs);
    if let Some(p) = pane {
        let widget = p.widget_mut().text_mut().unwrap();

        let is_bottom = widget.is_bottom();

        widget.append_items(&logs);

        if is_bottom {
            widget.select_last();
        }
    }
}

fn selected_pod(window: &Window) -> String {
    match window.pane(view_id::tab_pods_pane_pods) {
        Some(pane) => {
            let w = pane.widget().table().unwrap();
            let index = w.state().borrow().selected();

            w.items()[index.unwrap()][0].to_string()
        }
        None => String::new(),
    }
}

fn selected_config(window: &Window) -> String {
    let pane = window.pane(view_id::tab_configs_pane_configs).unwrap();
    let selected_index = pane
        .widget()
        .list()
        .unwrap()
        .state()
        .borrow()
        .selected()
        .unwrap();
    pane.widget().list().unwrap().items()[selected_index].clone()
}

fn update_window_pane_items(window: &mut Window, id: &str, items: WidgetItem) {
    let pane = window.pane_mut(id);
    if let Some(p) = pane {
        p.set_items(items);
    }
}

pub fn apis_subwin_action<'a, P>(
    _window: &mut Window,
    subwin: &mut SubWindow<P>,
    _tx: &Sender<Event>,
    rx: &Receiver<Event>,
) -> WindowEvent
where
    P: PaneTrait<Item = Select<'a>>,
{
    let pane = subwin.pane_mut();

    match rx.recv().unwrap() {
        Event::Input(key) => match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => {
                return WindowEvent::CloseSubWindow
            }

            KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_next_item();
            }

            KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_prev_item();
            }

            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_next_item();
            }

            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_prev_item();
            }

            KeyCode::Char('h') if key.modifiers == KeyModifiers::CONTROL => {
                pane.remove_char();
            }

            KeyCode::Tab => {
                pane.select_next_pane();
            }

            KeyCode::Enter | KeyCode::Char(' ') => {
                pane.toggle_select_unselect();
            }

            KeyCode::Delete | KeyCode::Backspace => {
                pane.remove_char();
            }

            KeyCode::Right => {
                pane.forward_cursor();
            }

            KeyCode::Left => {
                pane.back_cursor();
            }

            KeyCode::Char(c) => {
                pane.insert_char(c);
            }

            _ => {}
        },
        Event::Kube(k) => match k {
            Kube::GetAPIsResponse(apis) => pane.set_items(apis),
            _ => {}
        },
        Event::Resize(_w, _h) => {
            return WindowEvent::ResizeWindow;
        }
        _ => {}
    }

    WindowEvent::Continue
}

pub fn namespace_subwin_action<'a, P>(
    window: &mut Window,
    subwin: &mut SubWindow<P>,
    tx: &Sender<Event>,
    rx: &Receiver<Event>,
    current_namespace: &mut String,
) -> WindowEvent
where
    P: PaneTrait<Item = Pane<'a>>,
{
    let pane = subwin.pane_mut();
    match rx.recv().unwrap() {
        Event::Input(ev) => match ev.code {
            KeyCode::Char('q') => return WindowEvent::CloseSubWindow,
            KeyCode::Char('j') | KeyCode::Down => {
                pane.select_next_item(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                pane.select_prev_item(1);
            }
            KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
                pane.select_next_item(1);
            }
            KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
                pane.select_prev_item(1);
            }
            KeyCode::Char('u') if ev.modifiers == KeyModifiers::CONTROL => {
                pane.select_next_item(1);
            }
            KeyCode::Char('d') if ev.modifiers == KeyModifiers::CONTROL => {
                pane.select_prev_item(1);
            }

            KeyCode::Char('G') => {
                pane.select_last_item();
            }
            KeyCode::Char('g') => {
                pane.select_first_item();
            }

            KeyCode::Enter => {
                if let Some(item) = pane.get_item(view_id::subwin_ns_pane_ns) {
                    let item = item.get_simple();

                    tx.send(Event::Kube(Kube::SetNamespace(item.to_string())))
                        .unwrap();

                    *current_namespace = item.to_string();

                    if let Some(p) = window.pane_mut(view_id::tab_event_pane_event) {
                        p.clear();
                    }

                    if let Some(p) = window.pane_mut(view_id::tab_pods_pane_logs) {
                        p.clear();
                        window.select_pane(view_id::tab_pods_pane_pods);
                    }

                    if let Some(p) = window.pane_mut(view_id::tab_configs_pane_raw_data) {
                        p.clear();
                        window.select_pane(view_id::tab_configs_pane_configs);
                    }
                }
                return WindowEvent::CloseSubWindow;
            }
            _ => {}
        },
        Event::Kube(k) => match k {
            Kube::GetNamespacesResponse(ns) => pane.set_items(WidgetItem::Array(ns)),
            _ => {}
        },
        Event::Resize(_w, _h) => {
            return WindowEvent::ResizeWindow;
        }
        _ => {}
    }

    WindowEvent::Continue
}

pub fn window_action<P: PaneTrait>(
    window: &mut Window,
    _subwin: &mut SubWindow<P>,
    tx: &Sender<Event>,
    rx: &Receiver<Event>,
    current_namespace: &mut String,
    current_context: &mut String,
) -> WindowEvent {
    match rx.recv().unwrap() {
        Event::Input(ev) => match ev.code {
            KeyCode::Char('q') => {
                return WindowEvent::CloseWindow;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                window.select_next_item();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                window.select_prev_item();
            }
            KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
                window.select_next_item();
            }
            KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
                window.select_prev_item();
            }
            KeyCode::Char('u') if ev.modifiers == KeyModifiers::CONTROL => {
                window.scroll_up();
            }
            KeyCode::Char('d') if ev.modifiers == KeyModifiers::CONTROL => {
                window.scroll_down();
            }
            KeyCode::Tab if ev.modifiers == KeyModifiers::NONE => {
                window.select_next_pane();
            }
            KeyCode::BackTab | KeyCode::Tab if ev.modifiers == KeyModifiers::SHIFT => {
                window.select_prev_pane();
            }
            KeyCode::Char(n @ '1'..='9') => {
                window.select_tab(n as usize - b'0' as usize);
            }
            KeyCode::Char('n') => {
                tx.send(Event::Kube(Kube::GetNamespacesRequest)).unwrap();
                return WindowEvent::OpenSubWindow(view_id::subwin_ns);
            }
            KeyCode::Char('G') => {
                window.select_last_item();
            }
            KeyCode::Char('g') => {
                window.select_first_item();
            }

            KeyCode::Char('/') | KeyCode::Char('f') => {
                if window.selected_tab_id() == view_id::tab_apis {
                    tx.send(Event::Kube(Kube::GetAPIsRequest)).unwrap();
                    return WindowEvent::OpenSubWindow(view_id::subwin_apis);
                }
            }
            KeyCode::Enter => match window.selected_pane_id() {
                view_id::tab_pods_pane_pods => {
                    window.pane_clear(view_id::tab_pods_pane_logs);
                    tx.send(Event::Kube(Kube::LogStreamRequest(selected_pod(&window))))
                        .unwrap();
                }
                view_id::tab_configs_pane_configs => {
                    window.pane_clear(view_id::tab_configs_pane_configs);
                    tx.send(Event::Kube(Kube::ConfigRequest(selected_config(&window))))
                        .unwrap();
                }
                _ => {}
            },
            _ => {}
        },

        Event::Resize(_w, _h) => {
            return WindowEvent::ResizeWindow;
        }
        Event::Tick => {}
        Event::Mouse => {}
        Event::Kube(k) => match k {
            Kube::Pod(info) => {
                update_window_pane_items(
                    window,
                    view_id::tab_pods_pane_pods,
                    WidgetItem::DoubleArray(info),
                );
            }

            Kube::Configs(configs) => {
                update_window_pane_items(
                    window,
                    view_id::tab_configs_pane_configs,
                    WidgetItem::Array(configs),
                );
            }
            Kube::LogStreamResponse(logs) => {
                update_pod_logs(window, logs);
            }

            Kube::ConfigResponse(raw) => {
                update_window_pane_items(
                    window,
                    view_id::tab_configs_pane_raw_data,
                    WidgetItem::Array(raw),
                );
            }

            Kube::GetCurrentContextResponse(ctx, ns) => {
                *current_context = ctx;
                *current_namespace = ns;
            }
            Kube::Event(ev) => {
                update_event(window, ev);
            }
            _ => unreachable!(),
        },
    }
    WindowEvent::Continue
}
