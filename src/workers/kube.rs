pub mod color;
mod config;
mod controller;
pub mod message;
mod store;
mod worker;

pub use config::KubeWorkerConfig;
pub use controller::*;
pub use worker::*;

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use tokio::runtime::Runtime;

use crate::{logger, message::Message, panic_set_hook};

#[derive(Debug, Clone)]
pub struct KubeWorker {
    pub(super) tx: Sender<Message>,
    pub(super) rx: Receiver<Message>,
    pub(super) tx_shutdown: Sender<Result<()>>,
    pub(super) config: KubeWorkerConfig,
}

impl KubeWorker {
    pub fn new(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        tx_shutdown: Sender<Result<()>>,
        config: KubeWorkerConfig,
    ) -> Self {
        KubeWorker {
            tx,
            rx,
            tx_shutdown,
            config,
        }
    }

    pub fn start(self) {
        logger!(info, "KubeWorker start");

        let rt = Runtime::new().expect("failed to create runtime");

        let ret = rt.block_on(start_controller(self.tx, self.rx, self.config));

        if let Err(e) = &ret {
            logger!(error, "{}", e);
        }

        logger!(info, "KubeWorker end");

        self.tx_shutdown
            .send(ret)
            .expect("failed to send shutdown signal");
    }

    pub fn set_panic_hook(&self) {
        let tx_shutdown = self.tx_shutdown.clone();

        panic_set_hook!({
            tx_shutdown
                .send(Err(anyhow::anyhow!("panic occurred in KubeWorker worker")))
                .expect("failed to send shutdown signal");
        });
    }
}

async fn start_controller(
    tx: Sender<Message>,
    rx: Receiver<Message>,
    config: KubeWorkerConfig,
) -> Result<()> {
    let controller = KubeController::new(tx, rx, config).await?;
    controller.run().await
}
