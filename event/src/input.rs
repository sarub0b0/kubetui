use super::Event;

use crossbeam::channel::Sender;

use crossterm::event::{read, Event as CEvent};

pub fn read_key(tx: Sender<Event>) {
    loop {
        match read().unwrap() {
            CEvent::Key(ev) => tx.send(Event::Input(ev)).unwrap(),
            CEvent::Mouse(ev) => tx.send(Event::Mouse(ev)).unwrap(),
            CEvent::Resize(w, h) => tx.send(Event::Resize(w, h)).unwrap(),
        }
    }
}
