use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::panic_set_hook;

use super::*;
use anyhow::Result;
use crossbeam::channel::Sender;

use tokio::runtime::Runtime;
use tokio::time;

pub fn tick(tx: Sender<Event>, rate: time::Duration, is_terminated: Arc<AtomicBool>) -> Result<()> {
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

    #[cfg(feature = "logging")]
    log::debug!("Terminated tick event");

    ret
}
