use super::super::bot::Error;
use super::AudioStream;
use super::Regexp;
use super::Search;

use regex::Regex;
use reqwest::Client as HttpClient;
use serenity::async_trait;
use songbird::input::Input;
use songbird::input::YoutubeDl;
use std::io::Error as StdErr;

const SOUNDCLOUD_REGEX: &'static str =
    r"(?:https?:\/\/)?(?:www\.)?soundcloud\.com\/([\w-]+)\/([\w-]+)";

pub struct SoundCloudProvider {}

impl AudioStream for SoundCloudProvider {
    fn get_stream(&self, http_client: HttpClient, url: String) -> Input {
        let request = YoutubeDl::new(http_client, url);
        return Input::from(request);
    }
}

#[async_trait]
impl Search for SoundCloudProvider {
    async fn search(&self, query: &str) -> Result<Vec<String>, Error> {
        Err(Box::new(StdErr::new(
            std::io::ErrorKind::Other,
            "Not implemented yet",
        )))
    }
}

impl Regexp for SoundCloudProvider {
    fn is_valid(&self, input: &str) -> bool {
        return Regex::new(SOUNDCLOUD_REGEX)
            .expect("Youtube regex failed to compile")
            .is_match(input);
    }
}
