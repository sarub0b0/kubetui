use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::panic_set_hook;

use super::*;

use anyhow::Result;
use crossbeam::channel::Sender;

use crossterm::event::{poll, read, Event as CEvent};

pub fn read_key(tx: Sender<Event>, is_terminated: Arc<AtomicBool>) -> Result<()> {
    let is_terminated_panic = is_terminated.clone();
    panic_set_hook!({
        is_terminated_panic.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let ret: Result<()> = {
        while !is_terminated.load(Ordering::Relaxed) {
            if let Ok(true) = poll(Duration::from_secs(1)) {
                let ev = read()?;

                #[cfg(feature = "logging")]
                log::debug!("{:?}", ev);

                match ev {
                    CEvent::Key(ev) => tx.send(Event::User(UserEvent::Key(ev)))?,
                    CEvent::Mouse(ev) => tx.send(Event::User(UserEvent::Mouse(ev)))?,
                    CEvent::Resize(w, h) => tx.send(Event::User(UserEvent::Resize(w, h)))?,
                }
            }
        }

        Ok(())
    };

    is_terminated.store(true, std::sync::atomic::Ordering::Relaxed);

    #[cfg(feature = "logging")]
    log::debug!("Terminated read-key event");

    ret
}
