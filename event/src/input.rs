use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use super::Event;
use crate::UserEvent;

use crossbeam::channel::Sender;

use crossterm::event::{poll, read, Event as CEvent};

pub fn read_key(tx: Sender<Event>, is_terminated: Arc<AtomicBool>) {
    while !is_terminated.load(Ordering::Relaxed) {
        if let Ok(true) = poll(Duration::from_secs(1)) {
            match read() {
                Ok(ev) => {
                    #[cfg(feature = "logging")]
                    log::debug!("{:?}", ev);

                    match ev {
                        CEvent::Key(ev) => tx.send(Event::User(UserEvent::Key(ev))).unwrap(),
                        CEvent::Mouse(ev) => tx.send(Event::User(UserEvent::Mouse(ev))).unwrap(),
                        CEvent::Resize(w, h) => {
                            tx.send(Event::User(UserEvent::Resize(w, h))).unwrap()
                        }
                    }
                }
                Err(_) => {}
            }
        } else {
        }
    }

    #[cfg(feature = "logging")]
    log::debug!("Terminated read-key event");
}
