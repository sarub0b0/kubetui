use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{sleep, spawn, JoinHandle},
};

use crate::{event::Event, logger, panic_set_hook};

use anyhow::Result;
use crossbeam::channel::Sender;
use tokio::time;

pub struct Tick {
    tx: Sender<Event>,
    duration: time::Duration,
    is_terminated: Arc<AtomicBool>,
}

impl Tick {
    pub fn new(tx: Sender<Event>, rate: time::Duration, is_terminated: Arc<AtomicBool>) -> Self {
        Self {
            tx,
            duration: rate,
            is_terminated,
        }
    }

    pub fn start(self) -> JoinHandle<Result<()>> {
        logger!(info, "Start tick event");

        let handle = spawn(move || {
            self.set_panic_hook();

            let is_terminated = self.is_terminated.clone();

            let ret = self.tick();

            is_terminated.store(true, Ordering::Relaxed);

            ret
        });

        logger!(info, "Terminated tick event");

        handle
    }

    fn set_panic_hook(&self) {
        let is_terminated = self.is_terminated.clone();

        panic_set_hook!({
            is_terminated.store(true, Ordering::Relaxed);
        });
    }

    fn tick(&self) -> Result<()> {
        while !self.is_terminated.load(Ordering::Relaxed) {
            sleep(self.duration);

            self.tx.send(Event::Tick)?;
        }

        Ok(())
    }
}
