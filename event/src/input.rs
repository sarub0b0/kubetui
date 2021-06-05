use crate::UserEvent;

use super::Event;

use crossbeam::channel::Sender;

use crossterm::event::{read, Event as CEvent};

pub fn read_key(tx: Sender<Event>) {
    loop {
        match read().unwrap() {
            CEvent::Key(ev) => tx.send(Event::User(UserEvent::Key(ev))).unwrap(),
            CEvent::Mouse(ev) => tx.send(Event::User(UserEvent::Mouse(ev))).unwrap(),
            CEvent::Resize(w, h) => tx.send(Event::User(UserEvent::Resize(w, h))).unwrap(),
        }
    }
}
