use std::{
    sync::{atomic::AtomicBool, Arc},
    thread, time,
};

use anyhow::{Context as _, Result};
use crossbeam::channel::{bounded, Receiver, Sender};

use crate::{
    cmd::Command,
    message::Message,
    workers::{KubeWorker, Render, Tick, UserInput},
};

pub struct App;

impl App {
    pub fn run(cmd: Command) -> Result<()> {
        let split_direction = cmd.split_direction();
        let kube_worker_config = cmd.kube_worker_config();

        let (tx_input, rx_main): (Sender<Message>, Receiver<Message>) = bounded(128);
        let (tx_main, rx_kube): (Sender<Message>, Receiver<Message>) = bounded(256);
        let tx_kube = tx_input.clone();
        let tx_tick = tx_input.clone();

        let is_terminated = Arc::new(AtomicBool::new(false));

        let user_input = UserInput::new(tx_input.clone(), is_terminated.clone());

        let kube = KubeWorker::new(
            tx_kube.clone(),
            rx_kube.clone(),
            is_terminated.clone(),
            kube_worker_config,
        );

        let tick = Tick::new(
            tx_tick.clone(),
            time::Duration::from_millis(200),
            is_terminated.clone(),
        );

        let render = Render::new(
            tx_main.clone(),
            rx_main.clone(),
            is_terminated.clone(),
            split_direction,
        );

        thread::scope(|s| {
            let kube_handler = s.spawn(|| {
                kube.set_panic_hook();
                kube.start()
            });

            let tick_handler = s.spawn(move || {
                tick.set_panic_hook();
                tick.start()
            });

            let user_input_handler = s.spawn(move || {
                user_input.set_panic_hook();
                user_input.start()
            });

            let render_handler = s.spawn(move || {
                render.set_panic_hook();
                render.start()
            });

            kube_handler
                .join()
                .expect("kube thread panicked")
                .context("kube thread error")?;

            tick_handler
                .join()
                .expect("tick thread panicked")
                .context("tick thread error")?;

            user_input_handler
                .join()
                .expect("user_input thread panicked")
                .context("user_input thread error")?;

            render_handler
                .join()
                .expect("render thread panicked")
                .context("render thread error")?;

            anyhow::Ok(())
        })?;

        Ok(())
    }
}
