use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{spawn, JoinHandle},
    time::Duration,
};

use anyhow::Result;
use crossbeam::channel::Sender;
use crossterm::event::{poll, read, Event as CEvent, KeyEvent, KeyEventKind};

use crate::{
    event::{Event, UserEvent},
    logger, panic_set_hook,
};

/// ユーザー入力を受け付けるワーカースレッドを生成する構造体
/// イベントデータはチャネルを介してメインスレッドに送信される
pub struct UserInput {
    tx: Sender<Event>,
    is_terminated: Arc<AtomicBool>,
}

impl UserInput {
    pub fn new(tx: Sender<Event>, is_terminated: Arc<AtomicBool>) -> Self {
        Self { tx, is_terminated }
    }

    pub fn start(self) -> JoinHandle<Result<()>> {
        logger!(info, "Start read-key event");

        let handle = spawn(move || {
            let is_terminated = self.is_terminated.clone();

            panic_set_hook!({
                is_terminated.store(true, Ordering::Relaxed);
            });

            let is_terminated = self.is_terminated.clone();

            let ret = self.poll();

            is_terminated.store(true, Ordering::Relaxed);

            ret
        });

        logger!(info, "Terminated read-key event");

        handle
    }

    fn poll(&self) -> Result<()> {
        while !self.is_terminated.load(Ordering::Relaxed) {
            if let Ok(true) = poll(Duration::from_secs(1)) {
                let ev = read()?;

                logger!(debug, "{:?}", ev);

                match ev {
                    CEvent::Key(ev) => {
                        if let KeyEvent {
                            kind: KeyEventKind::Press | KeyEventKind::Repeat,
                            ..
                        } = ev
                        {
                            self.tx.send(Event::User(UserEvent::Key(ev)))?
                        }
                    }
                    CEvent::Mouse(ev) => self.tx.send(Event::User(UserEvent::Mouse(ev)))?,
                    CEvent::Resize(..) => {}
                    CEvent::FocusGained => self.tx.send(UserEvent::FocusGained.into())?,
                    CEvent::FocusLost => self.tx.send(UserEvent::FocusLost.into())?,
                    CEvent::Paste(_) => {}
                }
            }
        }

        Ok(())
    }
}

