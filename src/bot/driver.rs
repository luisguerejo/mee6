use super::bot::Error;
use super::status::Status;
use serenity::all::GuildId;
use serenity::async_trait;
use songbird::input::Input;
use songbird::tracks::TrackHandle;
use songbird::{Call, Event, EventContext, EventHandler as VoiceEventHandler, Songbird};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tracing::{error, warn};

#[derive(Clone)]
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

    pub async fn player(&self, call: Arc<Mutex<Call>>) {
        let call = Arc::clone(&call);
        let notify = Arc::clone(&self.notify);
        let queue = Arc::clone(&self.queue);
        let status = Arc::clone(&self.status);
        let current_track = Arc::clone(&self.current_track);

        loop {
            notify.notified().await;
            let mut manager = call.lock().await;

            // Signal to break out of this task
            // instead of having to carry around a
            // Future to cancel or join on
            if manager.current_channel().is_none() {
                break;
            }

            let mut queue = queue.lock().await;
            if let Some(song) = queue.pop_front() {
                // Need to grab all associated locks
                // let mut manager = manager_handle.lock().await;
                let mut current = current_track.lock().await;

                // Set current track handle from the result of
                // play input
                let track_handle = manager.play_input(song);
                *current = Some(track_handle);

                // Atomically change the DriverStatus
                let mut driver = status.lock().await;
                *driver = Status::Playing;
            }
        }
    }

    pub async fn leave(&self, manager: Arc<Songbird>, guild_id: GuildId) -> Result<(), Error> {
        let mut queue = self.queue.lock().await;
        let mut current_track = self.current_track.lock().await;
        let mut status = self.status.lock().await;

        if let Some(call) = manager.get(guild_id) {
            let mut call = call.lock().await;
            if let Err(e) = call.leave().await {
                error!("Error leaving voice channel: {:?}", e);
                return Err(e.to_string().into());
            }
            queue.clear();

            if let Some(ref track) = *current_track {
                if let Err(e) = track.stop() {
                    panic!("Error stopping current track when leaving: {e}");
                }
            }
            *current_track = None;
            *status = Status::Disconnected;

            return Ok(());
        }

        Err("Currently not in a voice channel to leave".into())
    }

    pub async fn skip_current_track(&self) -> Result<(), Error> {
        let mut current_track = self.current_track.lock().await;

        if let Some(track) = &mut *current_track {
            match *self.status.lock().await {
                Status::Playing => {
                    track.stop()?;
                    *current_track = None
                }
                Status::Paused => {
                    self.notify.notify_one();
                }
                _ => warn!("Attempting to skip in a none supported state"),
            }
            return Ok(());
        }

        Err("There is nothing to skip".into())
    }

    pub async fn pause_current_track(&self) -> Result<(), Error> {
        let current_track = self.current_track.lock().await;

        if current_track.is_none() {
            return Err("There is no track to pause".into());
        }

        let input = current_track.as_ref().unwrap();
        if let Err(e) = input.pause() {
            error!("Error pausing track:{}", e);
            return Err("Error pausing track".into());
        }

        Ok(())
    }

    pub async fn unpause_current_track(&self) -> Result<(), Error> {
        let current_track = self.current_track.lock().await;

        if current_track.is_none() {
            return Err("There is no track to play".into());
        }

        let input = current_track.as_ref().unwrap();
        if let Err(e) = input.play() {
            error!("Error unpausing track:{}", e);
            return Err("Error unpausing track".into());
        }

        Ok(())
    }

    pub async fn enqueue_input(&self, input: Input) -> Result<(), Error> {
        let mut queue = self.queue.lock().await;
        let status = self.status.lock().await;

        match *status {
            Status::Idle => {
                queue.push_front(input);
                self.notify.notify_one();
            }
            Status::Playing | Status::Paused => {
                queue.push_back(input);
            }
            _ => {
                let err_msg = "Undefined behavior, should be not allowed to queue songs since Bot is not connected";
                error!(err_msg);
                return Err(err_msg.into());
            }
        }
        Ok(())
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
        } else if front.is_none() && *status == Status::Playing || *status == Status::Paused {
            *status = Status::Idle;
        }
        return None;
    }
}
