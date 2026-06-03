mod action;
mod window;

use std::{cell::RefCell, io, rc::Rc};

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use ratatui::{backend::CrosstermBackend, layout::Direction, Terminal, TerminalOptions, Viewport};

use crate::{
    cmd::ClipboardMode,
    config::theme::ThemeConfig,
    features::{
        config::{ConfigColumns, ConfigLabelColumn},
        network::{NetworkColumns, NetworkLabelColumn},
        node::{NodeColumns, NodeLabelColumn},
        pod::{PodColumns, PodLabelColumn},
    },
    kube::context::{Context, Namespace},
    logger,
    message::Message,
    panic_set_hook,
    ui::WindowAction,
};

use self::{
    action::{update_contents, window_action},
    window::WindowInit,
};

pub struct Render {
    tx: Sender<Message>,
    rx: Receiver<Message>,
    tx_shutdown: Sender<Result<()>>,
    direction: Direction,
    default_pod_columns: Option<PodColumns>,
    default_node_columns: Option<NodeColumns>,
    default_config_columns: ConfigColumns,
    default_network_columns: NetworkColumns,
    pod_label_columns: Vec<PodLabelColumn>,
    node_label_columns: Vec<NodeLabelColumn>,
    config_label_columns: Vec<ConfigLabelColumn>,
    network_label_columns: Vec<NetworkLabelColumn>,
    theme: ThemeConfig,
    clipboard_mode: ClipboardMode,
    log_max_lines: Option<usize>,
}

impl Render {
    pub fn new(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        tx_shutdown: Sender<Result<()>>,
        direction: Direction,
        default_pod_columns: Option<PodColumns>,
        default_node_columns: Option<NodeColumns>,
        default_config_columns: ConfigColumns,
        default_network_columns: NetworkColumns,
        pod_label_columns: Vec<PodLabelColumn>,
        node_label_columns: Vec<NodeLabelColumn>,
        config_label_columns: Vec<ConfigLabelColumn>,
        network_label_columns: Vec<NetworkLabelColumn>,
        theme: ThemeConfig,
        clipboard_mode: ClipboardMode,
        log_max_lines: Option<usize>,
    ) -> Self {
        Self {
            direction,
            tx,
            rx,
            tx_shutdown,
            default_pod_columns,
            default_node_columns,
            default_config_columns,
            default_network_columns,
            pod_label_columns,
            node_label_columns,
            config_label_columns,
            network_label_columns,
            theme,
            clipboard_mode,
            log_max_lines,
        }
    }

    pub fn start(self) {
        logger!(info, "render start");

        let ret = self.render();

        if let Err(e) = &ret {
            logger!(error, "{}", e);
        }

        logger!(info, "render end");

        self.tx_shutdown
            .send(ret)
            .expect("failed to send shutdown signal");
    }

    pub fn set_panic_hook(&self) {
        let tx_shutdown = self.tx_shutdown.clone();

        panic_set_hook!({
            tx_shutdown
                .send(Err(anyhow::anyhow!("panic occurred in Render worker")))
                .expect("failed to send shutdown signal");
        });
    }

    fn render(&self) -> Result<()> {
        let namespace = Rc::new(RefCell::new(Namespace::new()));
        let context = Rc::new(RefCell::new(Context::new()));

        let mut window = WindowInit::new(
            self.direction,
            self.tx.clone(),
            context.clone(),
            namespace.clone(),
            self.default_pod_columns.clone(),
            self.default_node_columns.clone(),
            self.default_config_columns.clone(),
            self.default_network_columns.clone(),
            self.pod_label_columns.clone(),
            self.node_label_columns.clone(),
            self.config_label_columns.clone(),
            self.network_label_columns.clone(),
            self.theme.clone(),
            self.clipboard_mode,
            self.log_max_lines,
        )
        .build();

        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            TerminalOptions {
                viewport: Viewport::Fullscreen,
            },
        )?;

        terminal.clear()?;

        loop {
            terminal.draw(|f| {
                window.render(f);
            })?;

            match window_action(&mut window, &self.rx) {
                WindowAction::Continue => {}
                WindowAction::CloseWindow => {
                    break;
                }
                WindowAction::UpdateContents(ev) => {
                    update_contents(
                        &mut window,
                        ev,
                        &mut context.borrow_mut(),
                        &mut namespace.borrow_mut(),
                    );
                }
            }
        }

        Ok(())
    }
}
