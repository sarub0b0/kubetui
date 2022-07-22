use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{logger, panic_set_hook};

use super::Event;
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

    let rt = Runtime::new()?;

    let is_terminated_rt = is_terminated.clone();

    let ret: Result<()> = rt.block_on(async move {
        let mut interval = time::interval(rate);

        while !is_terminated_rt.load(Ordering::Relaxed) {
            interval.tick().await;

            tx.send(Event::Tick)?;
        }

        Ok(())
    });

    is_terminated.store(true, std::sync::atomic::Ordering::Relaxed);

    logger!(info, "Terminated tick event");

    ret
}
