use regex::Regex;
use reqwest::Client as HttpClient;
use serenity::async_trait;
use songbird::input::Input;
use songbird::tracks::TrackHandle;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, RwLock};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type BotContext<'a> = poise::PrefixContext<'a, Bot, Error>;

#[derive(Debug, PartialEq)]
pub enum DriverStatus {
    Playing,
    Idle,
    Paused,
    Disconnected,
}

#[allow(non_snake_case)]
pub struct Bot {
    pub httpClient: HttpClient,
    pub youtubeRegex: Regex,
    pub soundcloudRegex: Regex,
    pub queue: Arc<Mutex<VecDeque<Input>>>,
    pub notify: Arc<Notify>,
    pub driverStatus: Arc<RwLock<DriverStatus>>,
    pub currentTrack: Arc<Mutex<Option<TrackHandle>>>,
}

pub struct TrackEventHandler {
    pub notify: Arc<tokio::sync::Notify>,
    pub queue: Arc<Mutex<VecDeque<Input>>>,
    pub driver: Arc<RwLock<DriverStatus>>,
}

#[async_trait]
impl VoiceEventHandler for TrackEventHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let queue = Arc::clone(&self.queue);
        let queue = queue.lock().await;

        let status = Arc::clone(&self.driver);
        let mut status = status.write().await;
        let front = queue.front();
        if front.is_some() {
            self.notify.notify_one();
        } else if front.is_none() && *status == DriverStatus::Playing
            || *status == DriverStatus::Paused
        {
            *status = DriverStatus::Idle;
        }
        return None;
    }
}

impl Bot {
    pub fn new() -> Self {
        Self {
            httpClient: HttpClient::new(),
            youtubeRegex: Regex::new(
                r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$"
            ).expect("error creating regex"),
            soundcloudRegex: Regex::new(r"(?:https?:\/\/)?(?:www\.)?soundcloud\.com\/([\w-]+)\/([\w-]+)").expect("Error creating soundcloud regex"),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            driverStatus: Arc::new(RwLock::new(DriverStatus::Disconnected)),
            currentTrack: Arc::new(Mutex::new(None))
        }
    }
}
