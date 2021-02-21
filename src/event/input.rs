use super::Event;

use std::sync::mpsc::Sender;

use crossterm::event::{read, Event as CEvent};

pub fn read_key(tx: Sender<Event>) {
    loop {
        match read().unwrap() {
            CEvent::Key(ev) => tx.send(Event::Input(ev)).unwrap(),
            CEvent::Mouse(_) => tx.send(Event::Mouse).unwrap(),
            CEvent::Resize(_, _) => tx.send(Event::Resize).unwrap(),
        }
    }
}
