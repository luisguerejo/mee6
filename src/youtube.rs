use songbird::input::YoutubeDl;
use serenity::model::id::UserId;

pub struct SongMessage{
    pub link: YoutubeDl,
    pub from: UserId
}
