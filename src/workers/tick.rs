use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
};

use crate::{logger, message::Message, panic_set_hook};

use anyhow::Result;
use crossbeam::channel::Sender;
use tokio::time;

pub struct Tick {
    tx: Sender<Message>,
    duration: time::Duration,
    is_terminated: Arc<AtomicBool>,
}

impl Tick {
    pub fn new(tx: Sender<Message>, rate: time::Duration, is_terminated: Arc<AtomicBool>) -> Self {
        Self {
            tx,
            duration: rate,
            is_terminated,
        }
    }

    pub fn start(&self) -> Result<()> {
        logger!(info, "tick start");

        let ret = self.tick();

        self.is_terminated.store(true, Ordering::Relaxed);

        logger!(info, "tick end");

        ret
    }

    pub fn set_panic_hook(&self) {
        let is_terminated = self.is_terminated.clone();

        panic_set_hook!({
            is_terminated.store(true, Ordering::Relaxed);
        });
    }

    fn tick(&self) -> Result<()> {
        while !self.is_terminated.load(Ordering::Relaxed) {
            sleep(self.duration);

            self.tx.send(Message::Tick)?;
        }

        Ok(())
    }
}
