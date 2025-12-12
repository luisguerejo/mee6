use crate::bot::providers::AudioStream;

use super::super::bot::Error;
use super::Regexp;
use super::Search;

use regex::Regex;
use reqwest::Client as HttpClient;
use serenity::async_trait;
use songbird::input::Input;
use songbird::input::YoutubeDl;
use thirtyfour::prelude::*;

const YOUTUBE_REGEX: &'static str =
    r"(?:https?://)?(?:www\.)?(?:youtube\.com/watch\?v=|youtu\.be/)([a-zA-Z0-9_-]{11})";

pub struct YouTubeProvider {}

impl AudioStream for YouTubeProvider {
    fn get_stream(&self, http_client: HttpClient, url: String) -> Input {
        let request = YoutubeDl::new(http_client, url);
        return Input::from(request);
    }
}

#[async_trait]
impl Search for YouTubeProvider {
    async fn search(&self, query: &str) -> Result<Vec<String>, Error> {
        println!("I was called");
        let base: String = format!("https://www.youtube.com/results?search_query={}", query);

        let mut caps = DesiredCapabilities::chrome();
        caps.set_headless().unwrap();

        let driver = WebDriver::new("http://localhost:4444", caps).await?;
        driver.goto(base).await?;

        let videos = driver
            .find_all(By::Css("ytd-video-renderer.ytd-item-section-renderer"))
            .await?;

        let mut results = Vec::new();
        for video in videos {
            let x = video.find(By::Id("thumbnail")).await?;
            results.push(format!(
                "https://youtube.com{}",
                x.attr("href").await?.unwrap()
            ));
        }

        driver.quit().await?;
        return Ok(results);
    }
}

impl Regexp for YouTubeProvider {
    fn is_valid(&self, input: &str) -> bool {
        return Regex::new(YOUTUBE_REGEX)
            .expect("Youtube regex failed to compile")
            .is_match(input);
    }
}
