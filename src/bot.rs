use reqwest::Client as HttpClient;
use regex::Regex;
use tokio::sync::{ Notify, Mutex, RwLock };
use std::sync::Arc;
use std::collections::{HashSet, VecDeque};
use serenity::model::user::User;
use crate::youtube::SongMessage;

#[derive(Debug, PartialEq)]
pub enum DriverStatus{
    Playing,
    Idle,
    Disconnected
}

#[allow(non_snake_case)]
pub struct Bot {
    pub httpClient: HttpClient,
    pub youtubeRegex: Regex,
    pub queue: Arc<Mutex<VecDeque<SongMessage>>>,
    pub notify: Arc<Notify>,
    pub ignoreList: RwLock<HashSet<User>>,
    pub driverStatus: Arc<RwLock<DriverStatus>>
}

impl Bot {
    pub fn new() -> Self {
        Self {
            httpClient: HttpClient::new(),
            youtubeRegex: Regex::new(
                r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$"
            ).expect("error creating regex"),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            ignoreList: RwLock::new(HashSet::new()),
            driverStatus: Arc::new(RwLock::new(DriverStatus::Disconnected))
        }
    }
}
