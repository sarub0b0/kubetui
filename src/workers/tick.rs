use std::thread::sleep;

use crate::{logger, message::Message, panic_set_hook};

use anyhow::Result;
use crossbeam::channel::Sender;
use tokio::time;

pub struct Tick {
    tx: Sender<Message>,
    duration: time::Duration,
    tx_shutdown: Sender<Result<()>>,
}

impl Tick {
    pub fn new(tx: Sender<Message>, rate: time::Duration, tx_shutdown: Sender<Result<()>>) -> Self {
        Self {
            tx,
            duration: rate,
            tx_shutdown,
        }
    }

    pub fn start(&self) {
        logger!(info, "tick start");

        let ret = self.tick();

        if let Err(e) = &ret {
            logger!(error, "{}", e);
        }

        logger!(info, "tick end");

        self.tx_shutdown
            .send(ret)
            .expect("failed to send shutdown signal");
    }

    pub fn set_panic_hook(&self) {
        let tx_shutdown = self.tx_shutdown.clone();

        panic_set_hook!({
            tx_shutdown
                .send(Err(anyhow::anyhow!("panic occurred in Tick worker")))
                .expect("failed to send shutdown signal");
        });
    }

    fn tick(&self) -> Result<()> {
        loop {
            sleep(self.duration);

            self.tx.send(Message::Tick)?;
        }
    }
}
