pub mod color;
mod config;
mod controller;
pub mod message;
mod store;
mod worker;

pub use config::KubeWorkerConfig;
pub use controller::*;
pub use worker::*;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use tokio::runtime::Runtime;

use crate::{logger, message::Message, panic_set_hook};

#[derive(Debug, Clone)]
pub struct KubeWorker {
    pub(super) tx: Sender<Message>,
    pub(super) rx: Receiver<Message>,
    pub(super) is_terminated: Arc<AtomicBool>,
    pub(super) config: KubeWorkerConfig,
}

impl KubeWorker {
    pub fn new(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        is_terminated: Arc<AtomicBool>,
        config: KubeWorkerConfig,
    ) -> Self {
        KubeWorker {
            tx,
            rx,
            is_terminated,
            config,
        }
    }

    pub fn start(self) -> Result<()> {
        logger!(info, "KubeWorker start");

        let rt = Runtime::new()?;

        let is_terminated = self.is_terminated.clone();
        let ret = rt.block_on(start_controller(
            self.tx,
            self.rx,
            is_terminated,
            self.config,
        ));

        logger!(info, "KubeWorker end");

        if let Err(e) = ret {
            self.is_terminated.store(true, Ordering::Relaxed);

            Err(e)
        } else {
            Ok(())
        }
    }

    pub fn set_panic_hook(&self) {
        let is_terminated = self.is_terminated.clone();

        panic_set_hook!({
            is_terminated.store(true, Ordering::Relaxed);
        });
    }
}

async fn start_controller(
    tx: Sender<Message>,
    rx: Receiver<Message>,
    is_terminated: Arc<AtomicBool>,
    config: KubeWorkerConfig,
) -> Result<()> {
    let controller = KubeController::new(tx, rx, is_terminated, config).await?;
    controller.run().await
}
