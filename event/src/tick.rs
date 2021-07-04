use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use super::*;
use crossbeam::channel::Sender;

use tokio::runtime::Runtime;
use tokio::time;

pub fn tick(tx: Sender<Event>, rate: time::Duration, is_terminated: Arc<AtomicBool>) {
    let is_terminated_clone = is_terminated.clone();
    panic_set_hook!({
        is_terminated_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let mut interval = time::interval(rate);

        while !is_terminated.load(Ordering::Relaxed) {
            interval.tick().await;

            tx.send(Event::Tick).unwrap();
        }
    });

    #[cfg(feature = "logging")]
    log::debug!("Terminated tick event");
}
