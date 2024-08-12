use crate::youtube::SongMessage;
use regex::Regex;
use reqwest::Client as HttpClient;
use serenity::model::user::User;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, RwLock};

#[derive(Debug, PartialEq)]
pub enum DriverStatus {
    Playing,
    Idle,
    Disconnected,
}


#[derive(Debug, PartialEq)]
pub enum QueueRequest {
    Soundcloud,
    YoutubeURL,
    YoutubeQuery
}

pub fn query(arg: String) -> QueueRequest{

        let youtube = Regex::new(
            r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$"
            ).expect("error creating regex");
        let soundcloud = Regex::new(r"^(https?:\/\/)?(www.)?(m\.)?soundcloud\.com\/[\w\-\.]+(\/)+[\w\-\.]+/?$")
            .expect("Error creating regex");

        if youtube.is_match(&arg){
            return QueueRequest::YoutubeURL;
        }else if soundcloud.is_match(&arg){
            return QueueRequest::Soundcloud
        }else{
            return QueueRequest::YoutubeQuery
        }

}

#[allow(non_snake_case)]
pub struct Bot {
    pub httpClient: HttpClient,
    pub queue: Arc<Mutex<VecDeque<SongMessage>>>,
    pub notify: Arc<Notify>,
    pub ignoreList: RwLock<HashSet<User>>,
    pub driverStatus: Arc<RwLock<DriverStatus>>,
}

impl Bot {
    pub fn new() -> Self {
        Self {
            httpClient: HttpClient::new(),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            ignoreList: RwLock::new(HashSet::new()),
            driverStatus: Arc::new(RwLock::new(DriverStatus::Disconnected)),
        }
    }
}
