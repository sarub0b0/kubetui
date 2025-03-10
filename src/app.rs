use std::{thread, time};

use anyhow::Result;
use crossbeam::channel::{bounded, Receiver, Sender};

use crate::{
    cmd::Command,
    config::Config,
    features::{api_resources::kube::ApiConfig, event::kube::EventConfig, pod::kube::PodConfig},
    logger,
    message::Message,
    workers::{kube::YamlConfig, ApisConfig, KubeWorker, Render, Tick, UserInput},
};

pub struct App;

impl App {
    pub fn run(cmd: Command, config: Config) -> Result<()> {
        let split_direction = cmd.split_direction();
        let mut kube_worker_config = cmd.kube_worker_config();

        let (tx_input, rx_main): (Sender<Message>, Receiver<Message>) = bounded(128);
        let (tx_main, rx_kube): (Sender<Message>, Receiver<Message>) = bounded(256);
        let tx_kube = tx_input.clone();
        let tx_tick = tx_input.clone();

        let (tx_shutdown, rx_shutdown) = bounded::<()>(1);

        let user_input = UserInput::new(tx_input.clone(), tx_shutdown.clone());

        kube_worker_config.pod_config = PodConfig::from(config.theme.clone());
        kube_worker_config.event_config = EventConfig::from(config.theme.clone());
        kube_worker_config.api_config = ApiConfig::from(config.theme.clone());
        kube_worker_config.apis_config = ApisConfig::from(config.theme.clone());
        kube_worker_config.yaml_config = YamlConfig::from(config.theme.clone());

        let kube = KubeWorker::new(
            tx_kube.clone(),
            rx_kube.clone(),
            tx_shutdown.clone(),
            kube_worker_config,
        );

        let tick = Tick::new(
            tx_tick.clone(),
            time::Duration::from_millis(200),
            tx_shutdown.clone(),
        );

        let render = Render::new(
            tx_main.clone(),
            rx_main.clone(),
            tx_shutdown.clone(),
            split_direction,
            config.theme.clone(),
        );

        logger!(info, "app start");

        thread::spawn(|| {
            kube.set_panic_hook();
            kube.start();
        });

        thread::spawn(move || {
            tick.set_panic_hook();
            tick.start();
        });

        thread::spawn(move || {
            user_input.set_panic_hook();
            user_input.start();
        });

        thread::spawn(move || {
            render.set_panic_hook();
            render.start();
        });

        rx_shutdown.recv()?;

        logger!(info, "app end");

        Ok(())
    }
}
