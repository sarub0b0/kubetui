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
        config::WidgetConfig, MultipleSelect, SingleSelect, Table, Text, Widget, WidgetTrait,
    },
    Tab, Window, WindowEvent,
};

extern crate kubetui;
use kubetui::{
    action::{update_contents, view_id, window_action},
    config::{configure, Config},
    Context, Namespace,
};

#[cfg(feature = "logging")]
use kubetui::log::logging;

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

fn log_stream_request_param(value: &[String], namespace: &[String]) -> (String, String) {
    if namespace.len() == 1 {
        (namespace[0].to_string(), value[0].to_string())
    } else {
        (value[0].to_string(), value[1].to_string())
    }
}

#[inline]
fn init_pod(tx: Sender<Event>, namespace: Rc<RefCell<Namespace>>) -> Table<'static> {
    Table::builder()
        .id(view_id::tab_pods_widget_pods)
        .widget_config(&WidgetConfig::builder().title("Pods").build())
        .on_select(move |w, v| {
            w.widget_clear(view_id::tab_pods_widget_logs);

            let selected = &namespace.borrow().selected;

            let (ns, pod_name) = log_stream_request_param(v, selected);

            *(w.find_widget_mut(view_id::tab_pods_widget_logs)
                .widget_config_mut()
                .append_title_mut()) = Some((&pod_name).into());

            tx.send(Event::Kube(Kube::LogStreamRequest(ns, pod_name)))
                .unwrap();

            EventResult::Window(WindowEvent::Continue)
        })
        .build()
}

#[inline]
fn init_log(clipboard: Option<Rc<RefCell<ClipboardContextWrapper>>>) -> Text<'static> {
    let logs_builder = Text::builder()
        .id(view_id::tab_pods_widget_logs)
        .widget_config(&WidgetConfig::builder().title("Pods").build())
        .wrap()
        .follow();

    if let Some(cb) = &clipboard {
        logs_builder.clipboard(cb.clone())
    } else {
        logs_builder
    }
    .build()
}

fn config_request_param(value: &[String], namespace: &[String]) -> (String, String, String) {
    if namespace.len() == 1 {
        if 2 <= value.len() {
            (
                namespace[0].to_string(),
                value[0].to_string(),
                value[1].to_string(),
            )
        } else {
            (
                "Error".to_string(),
                "Error".to_string(),
                "Error".to_string(),
            )
        }
    } else if 3 <= value.len() {
        (
            value[0].to_string(),
            value[1].to_string(),
            value[2].to_string(),
        )
    } else {
        (
            "Error".to_string(),
            "Error".to_string(),
            "Error".to_string(),
        )
    }
}

#[inline]
fn init_configs(tx: Sender<Event>, namespace: Rc<RefCell<Namespace>>) -> Table<'static> {
    Table::builder()
        .id(view_id::tab_configs_widget_configs)
        .widget_config(&WidgetConfig::builder().title("Configs").build())
        .on_select(move |w, v| {
            w.widget_clear(view_id::tab_configs_widget_raw_data);

            let (ns, kind, name) = config_request_param(v, &namespace.borrow().selected);

            *(w.find_widget_mut(view_id::tab_configs_widget_raw_data)
                .widget_config_mut()
                .append_title_mut()) = Some((&name).into());

            tx.send(Event::Kube(Kube::ConfigRequest(ns, kind, name)))
                .unwrap();

            EventResult::Window(WindowEvent::Continue)
        })
        .build()
}

#[inline]
fn init_configs_raw(clipboard: Option<Rc<RefCell<ClipboardContextWrapper>>>) -> Text<'static> {
    let raw_data_builder = Text::builder()
        .id(view_id::tab_configs_widget_raw_data)
        .widget_config(&WidgetConfig::builder().title("Raw Data").build())
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
    context: Rc<RefCell<Context>>,
    namespace: Rc<RefCell<Namespace>>,
) -> SingleSelect<'static> {
    SingleSelect::builder()
        .id(view_id::subwin_ctx)
        .widget_config(&WidgetConfig::builder().title("Context").build())
        .on_select(move |w: &mut Window, v| {
            let item = v.to_string();

            tx.send(Event::Kube(Kube::SetContext(item.to_string())))
                .unwrap();

            let mut ctx = context.borrow_mut();
            ctx.update(item);

            let mut ns = namespace.borrow_mut();
            ns.selected = vec!["None".to_string()];

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
        .build()
}

#[inline]
fn init_subwin_single_ns(
    tx: Sender<Event>,
    namespace: Rc<RefCell<Namespace>>,
) -> SingleSelect<'static> {
    SingleSelect::builder()
        .id(view_id::subwin_single_ns)
        .widget_config(&WidgetConfig::builder().title("Namespace").build())
        .on_select(move |w: &mut Window, v| {
            let items = vec![v.to_string()];
            tx.send(Event::Kube(Kube::SetNamespaces(items.clone())))
                .unwrap();

            let mut ns = namespace.borrow_mut();
            ns.selected = items;

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
        .build()
}

#[inline]
fn init_subwin_multiple_ns(
    tx: Sender<Event>,
    namespace: Rc<RefCell<Namespace>>,
) -> MultipleSelect<'static> {
    MultipleSelect::builder()
        .id(view_id::subwin_ns)
        .widget_config(&WidgetConfig::builder().title("Namespace").build())
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
            ns.selected = items;

            w.widget_clear(view_id::tab_pods_widget_logs);
            w.widget_clear(view_id::tab_configs_widget_raw_data);
            w.widget_clear(view_id::tab_event_widget_event);
            w.widget_clear(view_id::tab_apis_widget_apis);

            EventResult::Nop
        })
        .build()
}

#[inline]
fn init_subwin_apis(tx: Sender<Event>) -> MultipleSelect<'static> {
    MultipleSelect::builder()
        .id(view_id::subwin_apis)
        .widget_config(&WidgetConfig::builder().title("APIs").build())
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
        .build()
}

type YamlState = Rc<RefCell<(String, String)>>;
#[inline]
fn init_subwin_yaml_kind(tx: Sender<Event>, state: YamlState) -> SingleSelect<'static> {
    SingleSelect::builder()
        .id(view_id::subwin_yaml_kind)
        .widget_config(&WidgetConfig::builder().title("Kind").build())
        .on_select(move |w, v| {
            #[cfg(feature = "logging")]
            ::log::info!("[subwin_yaml_kind] Select Item: {}", v);

            w.close_popup();

            let mut state = state.borrow_mut();
            state.0 = v.to_string();

            tx.send(Event::Kube(Kube::YamlResourceRequest(v.to_string())))
                .unwrap();

            w.open_popup(view_id::subwin_yaml_name);

            EventResult::Nop
        })
        .build()
}

#[inline]
fn init_subwin_yaml_name(
    tx: Sender<Event>,
    state: YamlState,
    namespace: Rc<RefCell<Namespace>>,
) -> SingleSelect<'static> {
    SingleSelect::builder()
        .id(view_id::subwin_yaml_name)
        .widget_config(&WidgetConfig::builder().title("Name").build())
        .on_select(move |w, v| {
            #[cfg(feature = "logging")]
            ::log::info!("[subwin_yaml_name] Select Item: {}", v);

            w.close_popup();

            let ns = &namespace.borrow().selected;

            let value: Vec<&str> = v.split_whitespace().collect();

            let (name, ns) = if value.len() == 1 {
                (value[0].to_string(), ns[0].to_string())
            } else {
                (value[1].to_string(), value[0].to_string())
            };

            let state = state.borrow();

            let kind = state.0.to_string();

            tx.send(Event::Kube(Kube::YamlRawRequest(kind, name, ns)))
                .unwrap();

            EventResult::Nop
        })
        .build()
}

fn init_window(
    split_mode: Direction,
    tx: Sender<Event>,
    context: Rc<RefCell<Context>>,
    namespaces: Rc<RefCell<Namespace>>,
) -> Window<'static> {
    let clipboard = match clipboard_wrapper::ClipboardContextWrapper::new() {
        Ok(cb) => Some(Rc::new(RefCell::new(cb))),
        Err(_) => None,
    };

    // Pods
    let pods_widget = init_pod(tx.clone(), namespaces.clone());

    // Logs
    let logs_widget = init_log(clipboard.clone());

    // Raw
    let configs_widget = init_configs(tx.clone(), namespaces.clone());
    let raw_data_widget = init_configs_raw(clipboard);

    // Event
    let event_widget = Text::builder()
        .id(view_id::tab_event_widget_event)
        .widget_config(&WidgetConfig::builder().title("Event").build())
        .wrap()
        .follow()
        .build();

    // APIs
    let tx_apis = tx.clone();
    let open_subwin = move |w: &mut Window| {
        tx_apis.send(Event::Kube(Kube::GetAPIsRequest)).unwrap();
        w.open_popup(view_id::subwin_apis);
        EventResult::Nop
    };

    let apis_widget = Text::builder()
        .id(view_id::tab_apis_widget_apis)
        .widget_config(&WidgetConfig::builder().title("APIs").build())
        .action('/', open_subwin.clone())
        .action('f', open_subwin)
        .build();

    // Yaml
    let yaml_state = Rc::new(RefCell::new((String::default(), String::default())));
    let tx_yaml = tx.clone();
    let state = yaml_state.clone();
    let open_subwin = move |w: &mut Window| {
        let mut state = state.borrow_mut();
        *state = (String::default(), String::default());

        tx_yaml.send(Event::Kube(Kube::YamlAPIsRequest)).unwrap();
        w.open_popup(view_id::subwin_yaml_kind);
        EventResult::Nop
    };

    let yaml_widget = Text::builder()
        .id(view_id::tab_yaml_widget_yaml)
        .widget_config(&WidgetConfig::builder().title("Yaml").build())
        .action('/', open_subwin.clone())
        .action('f', open_subwin)
        .wrap()
        .build();

    // [Sub Window] Context
    let subwin_ctx = Widget::from(init_subwin_ctx(tx.clone(), context, namespaces.clone()));

    // [Sub Window] Namespace (Single Select)
    let subwin_single_ns = Widget::from(init_subwin_single_ns(tx.clone(), namespaces.clone()));

    // [Sub Window] Namespace (Multiple Select)
    let subwin_multi_ns = Widget::from(init_subwin_multiple_ns(tx.clone(), namespaces.clone()));

    // [Sub Window] Api
    let subwin_apis = Widget::from(init_subwin_apis(tx.clone()));

    // [Sub Window] Yaml 1
    let subwin_yaml_kind = Widget::from(init_subwin_yaml_kind(tx.clone(), yaml_state.clone()));

    // [Sub Window] Yaml 2
    let subwin_yaml_name = Widget::from(init_subwin_yaml_name(tx.clone(), yaml_state, namespaces));

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
                .direction(split_mode.clone())
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
                .direction(split_mode)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            view_id::tab_event,
            "3:Event",
            [WidgetData::new(event_widget)],
        ),
        Tab::new(view_id::tab_apis, "4:APIs", [WidgetData::new(apis_widget)]),
        Tab::new(view_id::tab_yaml, "5:Yaml", [WidgetData::new(yaml_widget)]),
    ];

    let mut window = Window::new(tabs).status_target_id([
        (view_id::tab_pods, view_id::tab_pods_widget_logs),
        (view_id::tab_configs, view_id::tab_configs_widget_raw_data),
        (view_id::tab_event, view_id::tab_event_widget_event),
        (view_id::tab_apis, view_id::tab_apis_widget_apis),
        (view_id::tab_yaml, view_id::tab_yaml_widget_yaml),
    ]);

    // Configure Action
    let tx_clone = tx.clone();
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

    let tx_clone = tx.clone();
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

    let tx_clone = tx;
    window.add_action('c', move |w| {
        tx_clone
            .send(Event::Kube(Kube::GetContextsRequest))
            .unwrap();
        w.open_popup(view_id::subwin_ctx);
        EventResult::Nop
    });

    window.add_action('q', fn_close);
    window.add_action(KeyCode::Esc, fn_close);
    window.add_popup([
        subwin_multi_ns,
        subwin_apis,
        subwin_single_ns,
        subwin_ctx,
        subwin_yaml_kind,
        subwin_yaml_name,
    ]);

    window
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

    let namespace = Rc::new(RefCell::new(Namespace::new()));
    let context = Rc::new(RefCell::new(Context::new()));

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

    let mut window = init_window(
        config.split_mode(),
        tx_main,
        context.clone(),
        namespace.clone(),
    );

    terminal.clear()?;
    window.update_chunks(terminal.size()?);

    loop {
        terminal.draw(|f| {
            window.render(f, context.borrow(), namespace.borrow());
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
                    &mut context.borrow_mut(),
                    &mut namespace.borrow_mut(),
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

#[cfg(test)]
mod tests {
    use super::*;

    mod pod {
        use super::*;

        #[test]
        fn log_stream_request_param_single_namespace() {
            let value = vec![
                "name".to_string(),
                "ready".to_string(),
                "status".to_string(),
                "age".to_string(),
            ];
            let namespace = vec!["ns".to_string()];

            let actual = log_stream_request_param(&value, &namespace);

            assert_eq!(("ns".to_string(), "name".to_string()), actual)
        }

        #[test]
        fn log_stream_request_param_multiple_namespaces() {
            let value = vec![
                "ns-1".to_string(),
                "name".to_string(),
                "ready".to_string(),
                "status".to_string(),
                "age".to_string(),
            ];
            let namespace = vec!["ns-0".to_string(), "ns-1".to_string()];

            let actual = log_stream_request_param(&value, &namespace);

            assert_eq!(("ns-1".to_string(), "name".to_string()), actual)
        }
    }

    mod configs {
        use super::*;

        #[test]
        fn config_request_param_single_namespace() {
            let value = vec![
                "kind".to_string(),
                "name".to_string(),
                "data".to_string(),
                "age".to_string(),
            ];

            let namespace = vec!["ns".to_string()];

            let actual = config_request_param(&value, &namespace);

            assert_eq!(
                ("ns".to_string(), "kind".to_string(), "name".to_string()),
                actual
            )
        }

        #[test]
        fn config_request_param_multiple_namespaces() {
            let value = vec![
                "ns-1".to_string(),
                "kind".to_string(),
                "name".to_string(),
                "data".to_string(),
                "age".to_string(),
            ];
            let namespace = vec!["ns-0".to_string(), "ns-1".to_string()];
            let actual = config_request_param(&value, &namespace);

            assert_eq!(
                ("ns-1".to_string(), "kind".to_string(), "name".to_string()),
                actual
            )
        }
    }
}
