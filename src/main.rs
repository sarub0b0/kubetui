use clap::crate_name;
use crossbeam::channel::{unbounded, Receiver, Sender};
use event::UserEvent;
use std::{
    cell::RefCell,
    io, panic,
    rc::Rc,
    sync::{atomic::AtomicBool, Arc},
    thread, time,
};

use clipboard_wrapper::{ClipboardContextWrapper, ClipboardProvider};

use ::event::{error::Result, input::*, kubernetes::*, tick::*, Event};

use tui_wrapper::{
    crossterm::{
        cursor::Show,
        event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    event::EventResult,
    tab::WidgetData,
    tui::{
        backend::{Backend, CrosstermBackend},
        layout::{Constraint, Direction, Layout, Rect},
        Terminal, TerminalOptions, Viewport,
    },
    widget::{
        MultipleSelect, MultipleSelectBuilder, SingleSelect, SingleSelectBuilder, Table,
        TableBuilder, Text, TextBuilder, Widget, WidgetTrait,
    },
    Tab, Window, WindowEvent,
};

use clap::{crate_authors, crate_description, crate_version, App, Arg};

extern crate kubetui;
use kubetui::*;

#[derive(Debug)]
enum DirectionWrapper {
    Horizontal,
    Vertical,
}

impl Default for DirectionWrapper {
    fn default() -> Self {
        Self::Vertical
    }
}

#[derive(Debug, Default)]
struct Config {
    split_mode: DirectionWrapper,
}

impl Config {
    fn split_mode(&self) -> Direction {
        match self.split_mode {
            DirectionWrapper::Vertical => Direction::Vertical,
            DirectionWrapper::Horizontal => Direction::Horizontal,
        }
    }
}

macro_rules! enable_raw_mode {
    () => {
        enable_raw_mode().unwrap();
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
    };
}

macro_rules! disable_raw_mode {
    () => {
        execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show
        )
        .unwrap();
        disable_raw_mode().unwrap();
    };
}

#[inline]
fn init_pod(tx: Sender<Event>, selected_namespaces: Rc<RefCell<Vec<String>>>) -> Table<'static> {
    TableBuilder::default()
        .id(view_id::tab_pods_widget_pods)
        .title("Pods")
        .build()
        .on_select(move |w, v| {
            w.widget_clear(view_id::tab_pods_widget_logs);

            let (ns, pod_name) = if selected_namespaces.borrow().len() == 1 {
                (
                    selected_namespaces.borrow()[0].to_string(),
                    v[0].to_string(),
                )
            } else {
                (v[0].to_string(), v[1].to_string())
            };

            tx.send(Event::Kube(Kube::LogStreamRequest(ns, pod_name)))
                .unwrap();

            EventResult::Window(WindowEvent::Continue)
        })
}

#[inline]
fn init_log(clipboard: Option<Rc<RefCell<ClipboardContextWrapper>>>) -> Text<'static> {
    let logs_builder = TextBuilder::default()
        .id(view_id::tab_pods_widget_logs)
        .title("Logs")
        .wrap()
        .follow();

    if let Some(cb) = &clipboard {
        logs_builder.clipboard(cb.clone())
    } else {
        logs_builder
    }
    .build()
}

#[inline]
fn init_configs(
    tx: Sender<Event>,
    selected_namespaces: Rc<RefCell<Vec<String>>>,
) -> Table<'static> {
    TableBuilder::default()
        .id(view_id::tab_configs_widget_configs)
        .title("Configs")
        .build()
        .on_select(move |w, v| {
            w.widget_clear(view_id::tab_configs_widget_raw_data);

            let (ns, kind, name) = if selected_namespaces.borrow().len() == 1 {
                if 2 <= v.len() {
                    (
                        selected_namespaces.borrow()[0].to_string(),
                        v[0].to_string(),
                        v[1].to_string(),
                    )
                } else {
                    (
                        "Error".to_string(),
                        "Error".to_string(),
                        "Error".to_string(),
                    )
                }
            } else if 3 <= v.len() {
                (v[0].to_string(), v[1].to_string(), v[2].to_string())
            } else {
                (
                    "Error".to_string(),
                    "Error".to_string(),
                    "Error".to_string(),
                )
            };

            tx.send(Event::Kube(Kube::ConfigRequest(ns, kind, name)))
                .unwrap();

            EventResult::Window(WindowEvent::Continue)
        })
}

#[inline]
fn init_configs_raw(clipboard: Option<Rc<RefCell<ClipboardContextWrapper>>>) -> Text<'static> {
    let raw_data_builder = TextBuilder::default()
        .id(view_id::tab_configs_widget_raw_data)
        .title("Raw Data")
        .wrap();

    if let Some(cb) = clipboard {
        raw_data_builder.clipboard(cb)
    } else {
        raw_data_builder
    }
    .build()
}

#[inline]
fn init_subwin_ctx(
    tx: Sender<Event>,
    context: Rc<RefCell<String>>,
    namespace: Rc<RefCell<Vec<String>>>,
) -> SingleSelect<'static> {
    SingleSelectBuilder::default()
        .id(view_id::subwin_ctx)
        .title("Context")
        .build()
        .on_select(move |w: &mut Window, v| {
            let item = v.to_string();

            tx.send(Event::Kube(Kube::SetContext(item.to_string())))
                .unwrap();

            let mut ctx = context.borrow_mut();
            *ctx = item;

            let mut ns = namespace.borrow_mut();
            *ns = vec!["None".to_string()];

            w.close_popup();

            w.widget_clear(view_id::tab_pods_widget_logs);
            w.widget_clear(view_id::tab_configs_widget_raw_data);
            w.widget_clear(view_id::tab_event_widget_event);
            w.widget_clear(view_id::tab_apis_widget_apis);

            let widget = w
                .find_widget_mut(view_id::subwin_ns)
                .as_mut_multiple_select();

            widget.unselect_all();

            let widget = w
                .find_widget_mut(view_id::subwin_apis)
                .as_mut_multiple_select();

            widget.unselect_all();

            EventResult::Nop
        })
}

#[inline]
fn init_subwin_single_ns(
    tx: Sender<Event>,
    namespace: Rc<RefCell<Vec<String>>>,
) -> SingleSelect<'static> {
    SingleSelectBuilder::default()
        .id(view_id::subwin_single_ns)
        .title("Namespace")
        .build()
        .on_select(move |w: &mut Window, v| {
            let items = vec![v.to_string()];
            tx.send(Event::Kube(Kube::SetNamespaces(items.clone())))
                .unwrap();

            let mut ns = namespace.borrow_mut();
            *ns = items;

            w.close_popup();

            w.widget_clear(view_id::tab_pods_widget_logs);
            w.widget_clear(view_id::tab_configs_widget_raw_data);
            w.widget_clear(view_id::tab_event_widget_event);
            w.widget_clear(view_id::tab_apis_widget_apis);

            let widget = w
                .find_widget_mut(view_id::subwin_ns)
                .as_mut_multiple_select();

            widget.unselect_all();

            widget.select_item(v);

            EventResult::Nop
        })
}

#[inline]
fn init_subwin_multiple_ns(
    tx: Sender<Event>,
    namespace: Rc<RefCell<Vec<String>>>,
) -> MultipleSelect<'static> {
    MultipleSelectBuilder::default()
        .id(view_id::subwin_ns)
        .title("Namespace")
        .build()
        .on_select(move |w: &mut Window, _| {
            let widget = w
                .find_widget_mut(view_id::subwin_ns)
                .as_mut_multiple_select();

            widget.toggle_select_unselect();

            let mut items = widget.selected_items();
            if items.is_empty() {
                items = vec!["None".to_string()];
            }

            tx.send(Event::Kube(Kube::SetNamespaces(items.clone())))
                .unwrap();

            let mut ns = namespace.borrow_mut();
            *ns = items;

            w.widget_clear(view_id::tab_pods_widget_logs);
            w.widget_clear(view_id::tab_configs_widget_raw_data);
            w.widget_clear(view_id::tab_event_widget_event);
            w.widget_clear(view_id::tab_apis_widget_apis);

            EventResult::Nop
        })
}

#[inline]
fn init_subwin_apis(tx: Sender<Event>) -> MultipleSelect<'static> {
    MultipleSelectBuilder::default()
        .id(view_id::subwin_apis)
        .title("APIs")
        .build()
        .on_select(move |w, _| {
            let widget = w
                .find_widget_mut(view_id::subwin_apis)
                .as_mut_multiple_select();

            widget.toggle_select_unselect();

            if let Some(item) = widget.widget_item() {
                tx.send(Event::Kube(Kube::SetAPIsRequest(item.array())))
                    .unwrap();
            }

            if widget.selected_items().is_empty() {
                w.widget_clear(view_id::tab_apis_widget_apis)
            }

            EventResult::Nop
        })
}

fn run(config: Config) -> Result<()> {
    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = unbounded();
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = unbounded();
    let tx_kube = tx_input.clone();
    let tx_tick = tx_input.clone();

    let is_terminated = Arc::new(AtomicBool::new(false));

    let is_terminated_clone = is_terminated.clone();

    let read_key_handler = thread::spawn(move || read_key(tx_input, is_terminated_clone));

    let is_terminated_clone = is_terminated.clone();
    let kube_process_handler =
        thread::spawn(move || kube_process(tx_kube, rx_kube, is_terminated_clone));

    let is_terminated_clone = is_terminated.clone();
    let tick_handler = thread::spawn(move || {
        tick(
            tx_tick,
            time::Duration::from_millis(200),
            is_terminated_clone,
        )
    });

    let backend = CrosstermBackend::new(io::stdout());

    let mut current_namespace = "None".to_string();
    let selected_namespaces = Rc::new(RefCell::new(vec!["None".to_string()]));
    let current_context = Rc::new(RefCell::new("None".to_string()));

    let clipboard = match clipboard_wrapper::ClipboardContextWrapper::new() {
        Ok(cb) => Some(Rc::new(RefCell::new(cb))),
        Err(_) => None,
    };

    // TODO: 画面サイズ変更時にクラッシュする問題の解決
    //
    // Terminal::new()の場合は、terminal.draw実行時にautoresizeを実行してバッファを更新する。
    // そのため、リサイズイベント時に使用したサイズとterminal.draw実行時のサイズに差がでで
    // クラッシュすることがある。
    // 応急処置として、ドキュメントにはUNSTABLEとあるがdraw実行時のautoresizeを無効にする
    // オプションを使用する。
    //
    // UNSTABLE CODE
    let chunk = backend.size()?;
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::fixed(chunk),
        },
    )?;

    // Pods
    let pods_widget = init_pod(tx_main.clone(), selected_namespaces.clone());

    // Logs
    let logs_widget = init_log(clipboard.clone());

    // Raw
    let configs_widget = init_configs(tx_main.clone(), selected_namespaces.clone());
    let raw_data_widget = init_configs_raw(clipboard);

    // Event
    let event_widget = TextBuilder::default()
        .id(view_id::tab_event_widget_event)
        .title("Event")
        .wrap()
        .follow()
        .build();

    // APIs
    let mut apis_widget = TextBuilder::default()
        .id(view_id::tab_apis_widget_apis)
        .title("APIs")
        .build();

    let tx_apis = tx_main.clone();
    let open_subwin = move |w: &mut Window| {
        tx_apis.send(Event::Kube(Kube::GetAPIsRequest)).unwrap();
        w.open_popup(view_id::subwin_apis);
        EventResult::Nop
    };

    apis_widget.add_action('/', open_subwin.clone());
    apis_widget.add_action('f', open_subwin);

    // [Sub Window] Context
    let subwin_ctx = Widget::from(init_subwin_ctx(
        tx_main.clone(),
        current_context.clone(),
        selected_namespaces.clone(),
    ));

    // [Sub Window] Namespace (Single Select)
    let subwin_single_ns = Widget::from(init_subwin_single_ns(
        tx_main.clone(),
        selected_namespaces.clone(),
    ));

    // [Sub Window] Namespace (Multiple Select)
    let subwin_multi_ns = Widget::from(init_subwin_multiple_ns(
        tx_main.clone(),
        selected_namespaces.clone(),
    ));

    // [Sub Window] Api
    let subwin_apis = Widget::from(init_subwin_apis(tx_main.clone()));

    // Init Window
    let tabs = [
        Tab::new(
            view_id::tab_pods,
            "1:Pods",
            [
                WidgetData::new(pods_widget).chunk_index(0),
                WidgetData::new(logs_widget).chunk_index(1),
            ],
        )
        .layout(
            Layout::default()
                .direction(config.split_mode())
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            view_id::tab_configs,
            "2:Configs",
            [
                WidgetData::new(configs_widget).chunk_index(0),
                WidgetData::new(raw_data_widget).chunk_index(1),
            ],
        )
        .layout(
            Layout::default()
                .direction(config.split_mode())
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            view_id::tab_event,
            "3:Event",
            [WidgetData::new(event_widget)],
        ),
        Tab::new(view_id::tab_apis, "4:APIs", [WidgetData::new(apis_widget)]),
    ];

    let mut window = Window::new(tabs).status_target_id([
        (view_id::tab_pods, view_id::tab_pods_widget_logs),
        (view_id::tab_configs, view_id::tab_configs_widget_raw_data),
        (view_id::tab_event, view_id::tab_event_widget_event),
        (view_id::tab_apis, view_id::tab_apis_widget_apis),
    ]);

    // Configure Action
    let tx_clone = tx_main.clone();
    window.add_action(
        UserEvent::Key(KeyEvent {
            code: KeyCode::Char('N'),
            modifiers: KeyModifiers::SHIFT,
        }),
        move |w| {
            tx_clone
                .send(Event::Kube(Kube::GetNamespacesRequest))
                .unwrap();
            w.open_popup(view_id::subwin_ns);
            EventResult::Nop
        },
    );

    let tx_clone = tx_main.clone();
    window.add_action('n', move |w| {
        tx_clone
            .send(Event::Kube(Kube::GetNamespacesRequest))
            .unwrap();
        w.open_popup(view_id::subwin_single_ns);
        EventResult::Nop
    });

    let fn_close = |w: &mut Window| {
        if w.opening_popup() {
            w.close_popup();
            EventResult::Nop
        } else {
            EventResult::Window(WindowEvent::CloseWindow)
        }
    };

    let tx_clone = tx_main;
    window.add_action('c', move |w| {
        tx_clone
            .send(Event::Kube(Kube::GetContextsRequest))
            .unwrap();
        w.open_popup(view_id::subwin_ctx);
        EventResult::Nop
    });

    window.add_action('q', fn_close);
    window.add_action(KeyCode::Esc, fn_close);
    window.add_popup([subwin_multi_ns, subwin_apis, subwin_single_ns, subwin_ctx]);

    terminal.clear()?;
    window.update_chunks(terminal.size()?);

    loop {
        terminal.draw(|f| {
            window.render(f, &current_context.borrow(), &selected_namespaces.borrow());
        })?;

        match window_action(&mut window, &rx_main) {
            WindowEvent::Continue => {}
            WindowEvent::CloseWindow => {
                break;
            }
            WindowEvent::ResizeWindow(w, h) => {
                let chunk = Rect::new(0, 0, w, h);
                terminal.resize(chunk)?;
                window.update_chunks(chunk);
            }
            WindowEvent::UpdateContents(ev) => {
                update_contents(
                    &mut window,
                    ev,
                    &mut current_context.borrow_mut(),
                    &mut current_namespace,
                    &mut selected_namespaces.borrow_mut(),
                );
            }
        }
    }

    is_terminated.store(true, std::sync::atomic::Ordering::Relaxed);

    read_key_handler.join().unwrap();

    kube_process_handler
        .join()
        .unwrap_or_else(|e| *e.downcast().unwrap())?;

    tick_handler.join().unwrap();

    Ok(())
}

fn configure() -> Config {
    let app = App::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("split-mode")
                .short("s")
                .long("split-mode")
                .help("Window split mode")
                .value_name("direction")
                .default_value("vertical")
                .possible_values(&["vertical", "v", "horizontal", "h"])
                .takes_value(true),
        )
        .get_matches();

    let mut config = Config::default();

    if let Some(d) = app.value_of("split-mode") {
        match d {
            "vertical" | "v" => {
                config.split_mode = DirectionWrapper::Vertical;
            }
            "horizontal" | "h" => {
                config.split_mode = DirectionWrapper::Horizontal;
            }
            _ => {}
        }
    }

    config
}

#[cfg(feature = "logging")]
use log::LevelFilter;
#[cfg(feature = "logging")]
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config as LConfig, Root},
    encode::pattern::PatternEncoder,
};
#[cfg(feature = "logging")]
use std::env;
#[cfg(feature = "logging")]
use std::str::FromStr;

#[cfg(feature = "logging")]
fn logging() {
    let level_filter =
        LevelFilter::from_str(&env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
            .unwrap_or(LevelFilter::Info);

    let logfile = FileAppender::builder()
        .append(false)
        .encoder(Box::new(PatternEncoder::new("{h({l})} - {m}\n")))
        .build("log/output.log")
        .unwrap();

    let config = LConfig::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(level_filter))
        .unwrap();

    log4rs::init_config(config).unwrap();
}

fn main() -> Result<()> {
    #[cfg(feature = "logging")]
    logging();

    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        disable_raw_mode!();

        eprintln!("\x1b[31mPanic! disable raw mode\x1b[39m");

        default_hook(info);
    }));

    let config = configure();

    enable_raw_mode!();

    let result = run(config);

    disable_raw_mode!();

    result
}
