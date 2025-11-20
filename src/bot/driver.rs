use serenity::async_trait;
use songbird::input::Input;
use songbird::tracks::TrackHandle;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use super::status::Status;

pub struct Driver {
    current_track: Arc<Mutex<Option<TrackHandle>>>,
    status: Arc<Mutex<Status>>,
    queue: Arc<Mutex<VecDeque<Input>>>,
    notify: Arc<Notify>,
}

impl Driver {
    pub fn new() -> Self {
        Self {
            current_track: Arc::new(Mutex::new(None)),
            notify: Arc::new(Notify::new()),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            status: Arc::new(Mutex::new(Status::Disconnected)),
        }
    }

    pub fn get_status(&self) -> Arc<Mutex<Status>> {
        Arc::clone(&self.status)
    }
}

#[async_trait]
impl VoiceEventHandler for Driver {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let queue = Arc::clone(&self.queue);
        let queue = queue.lock().await;

        let status = Arc::clone(&self.status);
        let mut status = status.lock().await;
        let front = queue.front();
        if front.is_some() {
            self.notify.notify_one();
        } else if front.is_none() && *status == Status::Playing
            || *status == Status::Paused
        {
            *status = Status::Idle;
        }
        return None;
    }
}
