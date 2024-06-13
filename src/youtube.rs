use songbird::input::YoutubeDl;
use serenity::model::id::UserId;

#[derive(Debug)]
pub struct SongMessage{
    pub link: YoutubeDl,
    pub from: UserId
}
