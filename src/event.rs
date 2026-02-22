use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
    pause_tx: watch::Sender<bool>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();
        let (pause_tx, mut pause_rx) = watch::channel(false);

        tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick = tokio::time::interval(tick_rate);

            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        if tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    Some(Ok(evt)) = reader.next() => {
                        match evt {
                            CrosstermEvent::Key(key) => {
                                if tx.send(Event::Key(key)).is_err() {
                                    break;
                                }
                            }
                            CrosstermEvent::Resize(w, h) => {
                                if tx.send(Event::Resize(w, h)).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(()) = pause_rx.changed() => {
                        if *pause_rx.borrow() {
                            // Paused: drop reader, wait for resume
                            drop(reader);
                            loop {
                                if pause_rx.changed().await.is_err() {
                                    return;
                                }
                                if !*pause_rx.borrow() {
                                    break;
                                }
                            }
                            // Resumed: recreate reader
                            reader = EventStream::new();
                            tick.reset();
                        }
                    }
                }
            }
        });

        Self { rx, _tx, pause_tx }
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }

    /// Pause event reading (drop EventStream so editor can use stdin)
    pub fn pause(&self) {
        let _ = self.pause_tx.send(true);
    }

    /// Resume event reading after editor exits
    pub fn resume(&self) {
        let _ = self.pause_tx.send(false);
    }
}
