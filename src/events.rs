use crate::gh::{PrDetails, PrId, PullRequest};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    TabLoaded(usize, Vec<PullRequest>),
    TabError(usize, String),
    DetailLoaded(PrId, PrDetails),
    DetailError(PrId),
}

pub fn spawn_event_task(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        loop {
            if let Some(Ok(Event::Key(key))) = reader.next().await {
                if tx.send(AppEvent::Key(key)).is_err() {
                    break;
                }
            }
        }
    });
}

pub fn is_quit(key: &KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('q'),
            ..
        } | KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
    )
}
