use std::time::Duration;

use anyhow::Result;
use crossbeam::channel::Sender;
use ratatui::crossterm::event::{Event as CEvent, KeyEvent, KeyEventKind, poll, read};

use crate::{
    logger,
    message::{Message, UserEvent},
    panic_set_hook,
};

/// ユーザー入力を受け付けるワーカースレッドを生成する構造体
/// イベントデータはチャネルを介してメインスレッドに送信される
pub struct UserInput {
    tx: Sender<Message>,
    tx_shutdown: Sender<Result<()>>,
}

impl UserInput {
    pub fn new(tx: Sender<Message>, tx_shutdown: Sender<Result<()>>) -> Self {
        Self { tx, tx_shutdown }
    }

    pub fn start(&self) {
        logger!(info, "user_input start");

        let ret = self.poll();

        if let Err(e) = &ret {
            logger!(error, "{}", e);
        }

        logger!(info, "user_input end");

        self.tx_shutdown
            .send(ret)
            .expect("failed to send shutdown signal");
    }

    pub fn set_panic_hook(&self) {
        let tx_shutdown = self.tx_shutdown.clone();

        panic_set_hook!({
            tx_shutdown
                .send(Err(anyhow::anyhow!("panic occurred in UserInput worker")))
                .expect("failed to send shutdown signal");
        });
    }

    fn poll(&self) -> Result<()> {
        loop {
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
                            self.tx.send(Message::User(UserEvent::Key(ev)))?
                        }
                    }
                    CEvent::Mouse(ev) => self.tx.send(Message::User(UserEvent::Mouse(ev)))?,
                    CEvent::Resize(..) => {}
                    CEvent::FocusGained => self.tx.send(UserEvent::FocusGained.into())?,
                    CEvent::FocusLost => self.tx.send(UserEvent::FocusLost.into())?,
                    CEvent::Paste(_) => {}
                }
            }
        }
    }
}
