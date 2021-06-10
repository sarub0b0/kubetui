use crossbeam::channel::{unbounded, Receiver, Sender};
use std::{cell::RefCell, io, panic, rc::Rc, thread, time};

use clipboard_wrapper::ClipboardProvider;

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
    widget::{
        ComplexWidget, ListBuilder, MultipleSelectBuilder, SingleSelectBuilder, TableBuilder,
        TextBuilder, Widget, WidgetTrait,
    },
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

    let current_namespace = Rc::new(RefCell::new("None".to_string()));
    let current_context = Rc::new(RefCell::new("None".to_string()));

    let clipboard = match clipboard_wrapper::ClipboardContextWrapper::new() {
        Ok(cb) => Some(Rc::new(RefCell::new(cb))),
        Err(_) => None,
    };

    // TODO: 画面サイズ変更時にクラッシュする問題の解決
    //
    // Terminal::new()の場合は、teminal.draw実行時にautoresizeを実行してバッファを更新する。
    // そのため、リサイズイベント時に使用したサイズとterminal.draw実行時のサイズに差がでで
    // クラッシュすることがある。
    // 応急処置として、ドキュメントにはUNSTABLEとあるがdraw実行時のautoresizeを無効にする
    // オプションを使用する。
    //
    // UNSTABLE CODE
    let chunk = backend.size().unwrap();
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::fixed(chunk),
        },
    )
    .unwrap();

    // Pods
    let tx_pods = tx_main.clone();

    let pods_widget = TableBuilder::default()
        .id(view_id::tab_pods_widget_pods)
        .title("Pods")
        .header([
            "NAME".to_string(),
            "READY".to_string(),
            "STATUS".to_string(),
            "AGE".to_string(),
        ])
        .build()
        .on_select(move |w, v| {
            w.widget_clear(view_id::tab_pods_widget_logs);
            tx_pods
                .send(Event::Kube(Kube::LogStreamRequest(v[0].to_string())))
                .unwrap();

            EventResult::Window(WindowEvent::Continue)
        });

    let logs_builder = TextBuilder::default()
        .id(view_id::tab_pods_widget_logs)
        .title("Logs")
        .wrap()
        .follow();

    let logs_widget = if let Some(cb) = &clipboard {
        logs_builder.clipboard(cb.clone())
    } else {
        logs_builder
    }
    .build();

    // Raw
    let tx_configs = tx_main.clone();

    let configs_widget = ListBuilder::default()
        .id(view_id::tab_configs_widget_configs)
        .title("Configs")
        .build()
        .on_select(move |w, item| {
            if let Some(widget) = w.find_widget_mut(view_id::tab_configs_widget_raw_data) {
                widget.clear();
            }
            tx_configs
                .send(Event::Kube(Kube::ConfigRequest(item.to_string())))
                .unwrap();
            EventResult::Window(WindowEvent::Continue)
        });

    let raw_data_builder = TextBuilder::default()
        .id(view_id::tab_configs_widget_raw_data)
        .title("Raw Data")
        .wrap();

    let raw_data_widget = if let Some(cb) = clipboard {
        raw_data_builder.clipboard(cb)
    } else {
        raw_data_builder
    }
    .build();

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
                .direction(Direction::Vertical)
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
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            view_id::tab_event,
            "3:Event",
            [WidgetData::new(event_widget)],
        ),
        Tab::new(view_id::tab_apis, "4:APIs", [WidgetData::new(apis_widget)]),
    ];

    let tx_ns = tx_main.clone();
    let cn = current_namespace.clone();
    let subwin_namespace = Widget::from(
        SingleSelectBuilder::default()
            .id(view_id::subwin_ns)
            .title("Namespace")
            .build()
            .on_select(move |w: &mut Window, item: &String| {
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
            }),
    );

    let tx_apis = tx_main.clone();
    let subwin_apis = Widget::from(
        MultipleSelectBuilder::default()
            .id(view_id::subwin_apis)
            .title("APIs")
            .build()
            .on_select(move |w, _| {
                if let Some(widget) = w.find_widget_mut(view_id::subwin_apis) {
                    if let ComplexWidget::MultipleSelect(widget) = widget.as_mut_complex() {
                        widget.toggle_select_unselect();

                        if let Some(item) = widget.widget_item() {
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
    );

    let mut window = Window::new(tabs).status_target_id([
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
    window.add_popup([subwin_namespace, subwin_apis]);

    terminal.clear().unwrap();
    window.update_chunks(terminal.size().unwrap());
    tx_main
        .send(Event::Kube(Kube::GetCurrentContextRequest))
        .unwrap();

    loop {
        terminal
            .draw(|f| {
                window.render(f, &current_context.borrow(), &current_namespace.borrow());
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
            WindowEvent::UpdateContents(ev) => {
                update_contents(
                    &mut window,
                    ev,
                    &mut current_context.borrow_mut(),
                    &mut current_namespace.borrow_mut(),
                );
            }
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
