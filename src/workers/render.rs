mod action;
mod window;

use std::{
    cell::RefCell,
    io::{self},
    rc::Rc,
};

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use ratatui::{backend::CrosstermBackend, layout::Direction, Terminal, TerminalOptions, Viewport};

use crate::{
    config::theme::ThemeConfig,
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
    tx_shutdown: Sender<()>,
    direction: Direction,
    theme: ThemeConfig,
}

impl Render {
    pub fn new(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        tx_shutdown: Sender<()>,
        direction: Direction,
        theme: ThemeConfig,
    ) -> Self {
        Self {
            direction,
            tx,
            rx,
            tx_shutdown,
            theme,
        }
    }

    pub fn start(self) {
        logger!(info, "render start");

        if let Err(err) = self.render() {
            logger!(error, "{}", err);
        }

        logger!(info, "render end");

        self.tx_shutdown
            .send(())
            .expect("failed to send shutdown signal");
    }

    pub fn set_panic_hook(&self) {
        let tx_shutdown = self.tx_shutdown.clone();

        panic_set_hook!({
            tx_shutdown
                .send(())
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
            self.theme.clone(),
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
