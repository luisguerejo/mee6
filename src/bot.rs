use reqwest::Client as HttpClient;
use regex::Regex;
use tokio::sync::{ mpsc, Notify, Mutex };
use std::sync::Arc;
use std::collections::VecDeque;
use crate::youtube::SongMessage;

#[allow(non_snake_case)]
pub struct Bot {
    pub httpClient: HttpClient,
    pub youtubeRegex: Regex,
    pub sender: mpsc::UnboundedSender<SongMessage>,
    pub reciever: Arc<Mutex<mpsc::UnboundedReceiver<SongMessage>>>,
    pub queue: Arc<Mutex<VecDeque<SongMessage>>>,
    pub notify: Arc<Notify>,
}

impl Bot {
    // pub fn send(&self, song: SongMessage) {
    //     self.queue.lock().await.push_front(song);
    //
    //     self.notify.notify_one();
    // }
    //
    // pub async fn recv(&self) -> SongMessage {
    //     loop{
    //         if let Some(msg) = self.queue.lock().await.pop_front(){
    //             return msg;
    //         }
    //         self.notify.notified().await;
    //     }
    // }
    pub fn new() -> Self {
        let (tx, recvr) = tokio::sync::mpsc::unbounded_channel();
        return Self {
            httpClient: HttpClient::new(),
            youtubeRegex: Regex::new(
                r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$"
            ).expect("error creating regex"),
            sender: tx,
            reciever: Arc::new(Mutex::new(recvr)),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
        };
    }
}
