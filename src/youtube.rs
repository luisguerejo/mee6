use serenity::model::id::UserId;
use songbird::input::Input;

pub struct SongMessage {
    pub link: String,
    pub input: Input,
    pub from: UserId,
}
