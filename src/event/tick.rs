use super::*;
use std::sync::mpsc::Sender;

use tokio::runtime::Runtime;
use tokio::time;

pub fn tick(tx: Sender<Event>, rate: time::Duration) {
    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let mut interval = time::interval(rate);
        loop {
            interval.tick().await;

            tx.send(Event::Tick).unwrap();
        }
    });
}
