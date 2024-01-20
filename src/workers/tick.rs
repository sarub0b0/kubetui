use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{event::Event, logger, panic_set_hook};

use anyhow::Result;
use crossbeam::channel::Sender;

use tokio::runtime::Runtime;
use tokio::time;

pub fn tick(tx: Sender<Event>, rate: time::Duration, is_terminated: Arc<AtomicBool>) -> Result<()> {
    logger!(info, "Start tick event");

    let is_terminated_panic = is_terminated.clone();
    panic_set_hook!({
        is_terminated_panic.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let ret = inner(tx, rate, is_terminated.clone());

    is_terminated.store(true, std::sync::atomic::Ordering::Relaxed);

    logger!(info, "Terminated tick event");

    ret
}

fn inner(tx: Sender<Event>, rate: time::Duration, is_terminated: Arc<AtomicBool>) -> Result<()> {
    let rt = Runtime::new()?;

    rt.block_on(async move {
        let mut interval = time::interval(rate);

        while !is_terminated.load(Ordering::Relaxed) {
            interval.tick().await;

            tx.send(Event::Tick)?;
        }

        Ok(())
    })
}
