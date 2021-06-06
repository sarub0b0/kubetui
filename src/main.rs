use crossbeam::channel::{unbounded, Receiver, Sender};
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;
use std::thread;
use std::time;
use tui_wrapper::widget::ComplexWidget;
use tui_wrapper::widget::MultipleSelect;

use std::io;

use clipboard_wrapper::{ClipboardContextWrapper, ClipboardProvider};

use ::event::{input::*, kubernetes::*, tick::*, Event};
use tui_wrapper::{
    crossterm::{
        cursor::Show,
        event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
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
    widget::{complex::SingleSelect, List, Table, Text, Widget, WidgetItem, WidgetTrait},
    Tab, Window, WindowEvent,
};

extern crate kubetui;
use kubetui::*;

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

fn run() {
    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = unbounded();
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = unbounded();
    let tx_kube = tx_input.clone();
    let tx_tick = tx_input.clone();

    thread::spawn(move || read_key(tx_input));
    thread::spawn(move || kube_process(tx_kube, rx_kube));
    thread::spawn(move || tick(tx_tick, time::Duration::from_millis(200)));

    let backend = CrosstermBackend::new(io::stdout());
    let chunk = backend.size().unwrap();

    let current_namespace = Rc::new(RefCell::new("None".to_string()));
    let mut current_context = "None".to_string();

    // TODO WSLの時はclip.exeにデータを渡せるようにデータ構造を定義する
    let clipboard: Result<ClipboardContextWrapper, _> =
        clipboard_wrapper::ClipboardContextWrapper::new();
    // TODO: 画面サイズ変更時にクラッシュする問題の解決
    //
    // Terminal::new()の場合は、teminal.draw実行時にautoresizeを実行してバッファを更新する。
    // そのため、リサイズイベント時に使用したサイズとterminal.draw実行時のサイズに差がでで
    // クラッシュすることがある。
    // 応急処置として、ドキュメントにはUNSTABLEとあるがdraw実行時のautoresizeを無効にする
    // オプションを使用する。
    //
    // UNSTABLE CODE
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::fixed(chunk),
        },
    )
    .unwrap();

    let mut logs_widget = Text::new(vec![]).enable_wrap().enable_follow();
    let mut raw_data_widget = Text::new(vec![]).enable_wrap();

    if let Ok(cb) = clipboard {
        let cb = Rc::new(RefCell::new(cb));
        logs_widget = logs_widget.clipboard(cb.clone());
        raw_data_widget = raw_data_widget.clipboard(cb);
    }
    let mut apis_widget = Text::new(vec![]);

    let tx_apis = tx_main.clone();

    let open_subwin = move |w: &mut Window| {
        tx_apis.send(Event::Kube(Kube::GetAPIsRequest)).unwrap();
        w.open_popup(view_id::subwin_apis);
        EventResult::Nop
    };

    apis_widget.add_action('/', open_subwin.clone());
    apis_widget.add_action('f', open_subwin);

    let tx_configs = tx_main.clone();
    let tx_pods = tx_main.clone();

    let tabs = vec![
        Tab::new(
            view_id::tab_pods,
            "1:Pods",
            vec![
                WidgetData {
                    chunk_index: 0,
                    widget: Widget::Table(Box::new(
                        Table::new(
                            vec![vec![]],
                            vec![
                                "NAME".to_string(),
                                "READY".to_string(),
                                "STATUS".to_string(),
                                "AGE".to_string(),
                            ],
                        )
                        .on_select(move |w, v| {
                            w.widget_clear(view_id::tab_pods_widget_logs);
                            tx_pods
                                .send(Event::Kube(Kube::LogStreamRequest(v[0].to_string())))
                                .unwrap();

                            EventResult::Window(WindowEvent::Continue)
                        })
                        .set_id(view_id::tab_pods_widget_pods)
                        .set_title("Pods"),
                    )),
                },
                WidgetData {
                    chunk_index: 1,
                    widget: Widget::Text(Box::new(
                        logs_widget
                            .set_title("Logs")
                            .set_id(view_id::tab_pods_widget_logs),
                    )),
                },
            ],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            view_id::tab_configs,
            "2:Configs",
            vec![
                WidgetData {
                    chunk_index: 0,
                    widget: Widget::List(Box::new(
                        List::new(vec![])
                            .on_select(move |w, item| {
                                if let Some(widget) =
                                    w.find_widget_mut(view_id::tab_configs_widget_raw_data)
                                {
                                    widget.clear();
                                }
                                tx_configs
                                    .send(Event::Kube(Kube::ConfigRequest(item.to_string())))
                                    .unwrap();
                                EventResult::Window(WindowEvent::Continue)
                            })
                            .set_id(view_id::tab_configs_widget_configs)
                            .set_title("Configs"),
                    )),
                },
                WidgetData {
                    widget: Widget::Text(Box::new(
                        raw_data_widget
                            .set_id(view_id::tab_configs_widget_raw_data)
                            .set_title("Raw Data"),
                    )),
                    chunk_index: 1,
                },
            ],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            view_id::tab_event,
            "3:Event",
            vec![WidgetData {
                widget: Widget::Text(Box::new(
                    Text::new(vec![])
                        .enable_wrap()
                        .enable_follow()
                        .set_title("Event")
                        .set_id(view_id::tab_event_widget_event),
                )),
                chunk_index: 0,
            }],
            Layout::default().constraints([Constraint::Percentage(100)].as_ref()),
        ),
        Tab::new(
            view_id::tab_apis,
            "4:APIs",
            vec![WidgetData {
                widget: Widget::Text(Box::new(
                    apis_widget
                        .set_id(view_id::tab_apis_widget_apis)
                        .set_title("APIs"),
                )),
                chunk_index: 0,
            }],
            Layout::default().constraints([Constraint::Percentage(100)].as_ref()),
        ),
    ];

    let tx_ns = tx_main.clone();
    let cn = current_namespace.clone();
    let subwin_namespace = Widget::Complex(Box::new(ComplexWidget::from(
        SingleSelect::new(view_id::subwin_ns, "Namespace").on_select(
            move |w: &mut Window, item: &String| {
                tx_ns
                    .send(Event::Kube(Kube::SetNamespace(item.to_string())))
                    .unwrap();

                let mut ns = cn.borrow_mut();
                *ns = item.to_string();

                w.close_popup();

                w.widget_clear(view_id::tab_pods_widget_logs);
                w.widget_clear(view_id::tab_configs_widget_raw_data);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_apis_widget_apis);

                EventResult::Nop
            },
        ),
    )));

    let tx_apis = tx_main.clone();
    let subwin_apis = Widget::Complex(Box::new(ComplexWidget::from(
        MultipleSelect::new(view_id::subwin_apis, "APIs").on_select(move |w, _| {
            if let Some(widget) = w.find_widget_mut(view_id::subwin_apis) {
                if let ComplexWidget::MultipleSelect(widget) = widget.as_mut_complex() {
                    widget.toggle_select_unselect();

                    if let Some(item) = widget.get_item() {
                        tx_apis
                            .send(Event::Kube(Kube::SetAPIsRequest(item.array())))
                            .unwrap();
                    }

                    if widget.selected_items().is_empty() {
                        w.widget_clear(view_id::tab_apis_widget_apis)
                    }
                }
            }
            EventResult::Nop
        }),
    )));

    let mut window = Window::new(tabs).status_target_id(vec![
        (view_id::tab_pods, view_id::tab_pods_widget_logs),
        (view_id::tab_configs, view_id::tab_configs_widget_raw_data),
        (view_id::tab_event, view_id::tab_event_widget_event),
        (view_id::tab_apis, view_id::tab_apis_widget_apis),
    ]);

    let tx_ns = tx_main.clone();
    window.add_action('n', move |w| {
        tx_ns.send(Event::Kube(Kube::GetNamespacesRequest)).unwrap();
        w.open_popup(view_id::subwin_ns);
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
    window.add_action('q', fn_close);
    window.add_action(KeyCode::Esc, fn_close);

    window.add_popup(vec![subwin_namespace, subwin_apis]);

    terminal.clear().unwrap();

    window.update_chunks(terminal.size().unwrap());

    tx_main
        .send(Event::Kube(Kube::GetCurrentContextRequest))
        .unwrap();

    loop {
        terminal
            .draw(|f| {
                let ns: &str = &current_namespace.borrow();
                window.render(f, &current_context, ns);
            })
            .unwrap();

        match window_action(&mut window, &rx_main) {
            WindowEvent::Continue => {}
            WindowEvent::CloseWindow => {
                break;
            }
            WindowEvent::ResizeWindow(w, h) => {
                let chunk = Rect::new(0, 0, w, h);
                terminal.resize(chunk).unwrap();
                window.update_chunks(chunk);
            }
            WindowEvent::UpdateContents(kube_ev) => match kube_ev {
                Kube::Pod(info) => {
                    set_items_widget(
                        &mut window,
                        view_id::tab_pods_widget_pods,
                        WidgetItem::DoubleArray(info),
                    );
                }

                Kube::Configs(configs) => {
                    set_items_widget(
                        &mut window,
                        view_id::tab_configs_widget_configs,
                        WidgetItem::Array(configs),
                    );
                }
                Kube::LogStreamResponse(logs) => {
                    append_items_widget(
                        &mut window,
                        view_id::tab_pods_widget_logs,
                        WidgetItem::Array(logs),
                    );
                }

                Kube::ConfigResponse(raw) => {
                    set_items_widget(
                        &mut window,
                        view_id::tab_configs_widget_raw_data,
                        WidgetItem::Array(raw),
                    );
                }

                Kube::GetCurrentContextResponse(ctx, ns) => {
                    current_context = ctx;
                    let mut cn = current_namespace.borrow_mut();
                    *cn = ns;
                }
                Kube::Event(ev) => {
                    set_items_widget(
                        &mut window,
                        view_id::tab_event_widget_event,
                        WidgetItem::Array(ev),
                    );
                }
                Kube::APIsResults(apis) => {
                    set_items_widget(
                        &mut window,
                        view_id::tab_apis_widget_apis,
                        WidgetItem::Array(apis),
                    );
                }
                Kube::GetNamespacesResponse(ns) => {
                    set_items_widget(&mut window, view_id::subwin_ns, WidgetItem::Array(ns));
                }

                Kube::GetAPIsResponse(apis) => {
                    set_items_widget(&mut window, view_id::subwin_apis, WidgetItem::Array(apis));
                }
                _ => unreachable!(),
            },
        }
    }
}

fn main() {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        disable_raw_mode!();

        eprintln!("\x1b[31mPanic! disable raw mode\x1b[39m");

        default_hook(info);
    }));

    enable_raw_mode!();

    run();

    disable_raw_mode!();
}
