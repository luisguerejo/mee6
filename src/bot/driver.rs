use super::bot::Error;
use super::status::Status;
use serenity::all::GuildId;
use serenity::async_trait;
use songbird::input::Input;
use songbird::tracks::TrackHandle;
use songbird::{Call, Event, EventContext, EventHandler as VoiceEventHandler, Songbird};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tracing::{error, info, warn};

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

    pub async fn player(&self, call: Arc<tokio::sync::Mutex<Call>>) {
        let call = Arc::clone(&call);
        let notify = Arc::clone(&self.notify);
        let queue = Arc::clone(&self.queue);
        let status = Arc::clone(&self.status);
        let current_track = Arc::clone(&self.current_track);

        call.lock()
            .await
            .add_global_event(Event::Track(songbird::TrackEvent::End), self.clone());

        {
            // Use scopes to release locks
            // since we just need to use this mutex
            // one time out of the loop
            let mut status = status.lock().unwrap();
            *status = Status::Idle;
        }

        loop {
            notify.notified().await;
            let mut manager = call.lock().await;

            // Signal to break out of this task
            // instead of having to carry around a
            // Future to cancel or join on
            if manager.current_channel().is_none() {
                let mut status = status.lock().unwrap();
                *status = Status::Disconnected;
                break;
            }

            let mut queue = queue.lock().unwrap();
            let mut status = status.lock().unwrap();
            if let Some(song) = queue.pop_front() {
                // Need to grab all associated locks
                let mut current_track = current_track.lock().unwrap();
                let track_handle = manager.play_input(song);
                *current_track = Some(track_handle);
                *status = Status::Playing;
            } else {
                *status = Status::Idle;
            }
        }
    }

    pub async fn leave(&self, manager: Arc<Songbird>, guild_id: GuildId) -> Result<(), Error> {
        if let Some(call) = manager.get(guild_id) {
            let mut call = call.lock().await;
            if let Err(e) = call.leave().await {
                error!("Error leaving voice channel: {:?}", e);
                return Err(e.to_string().into());
            }

            let mut queue = self.queue.lock().unwrap();
            let mut current_track = self.current_track.lock().unwrap();
            queue.clear();

            if let Some(ref track) = *current_track {
                if let Err(e) = track.stop() {
                    error!("Error stopping current track when leaving: {e}");
                }
            }
            *current_track = None;
            self.notify.notify_one();

            return Ok(());
        }

        Err("Currently not in a voice channel to leave".into())
    }

    pub async fn skip_current_track(&self) -> Result<(), Error> {
        let mut current_track = self.current_track.lock().unwrap();

        if let Some(track) = &mut *current_track {
            match *self.status.lock().unwrap() {
                Status::Playing => {
                    track.stop()?;
                    *current_track = None
                }
                Status::Paused => {
                    self.notify.notify_one();
                }
                _ => error!("Attempting to skip in a none supported state"),
            }
            return Ok(());
        }

        Err("There is nothing to skip".into())
    }

    pub async fn pause_current_track(&self) -> Result<(), Error> {
        let current_track = self.current_track.lock().unwrap();
        let mut status = self.status.lock().unwrap();

        if current_track.is_none() {
            return Err("There is no track to pause".into());
        }

        let input = current_track.as_ref().unwrap();
        if let Err(e) = input.pause() {
            error!("Error pausing track:{}", e);
            return Err("Error pausing track".into());
        }
        *status = Status::Paused;

        Ok(())
    }

    pub async fn unpause_current_track(&self) -> Result<(), Error> {
        let current_track = self.current_track.lock().unwrap();

        if current_track.is_none() {
            return Err("There is no track to play".into());
        }

        let input = current_track.as_ref().unwrap();
        if let Err(e) = input.play() {
            let error_message = format!("Error unpausing track: {e}");
            error!(error_message);
            return Err(error_message.into());
        }

        Ok(())
    }

    pub async fn enqueue_input(&self, input: Input) -> Result<(), Error> {
        let mut queue = self.queue.lock().unwrap();
        let status = self.status.lock().unwrap();

        match *status {
            Status::Idle => {
                queue.push_front(input);
                self.notify.notify_one();
            }
            Status::Playing | Status::Paused => {
                queue.push_back(input);
            }
            Status::Disconnected => {
                return Err("Not connected in a voice channel, use !join to connect".into())
            }
        }
        Ok(())
    }
}

#[async_trait]
impl VoiceEventHandler for Driver {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let queue = Arc::clone(&self.queue);
        let queue = queue.lock().unwrap();

        let status = Arc::clone(&self.status);
        let mut status = status.lock().unwrap();
        let front = queue.front();
        if front.is_some() {
            self.notify.notify_one();
        } else if front.is_none() && *status == Status::Playing || *status == Status::Paused {
            *status = Status::Idle;
        }
        return None;
    }
}
