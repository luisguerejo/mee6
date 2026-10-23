use crate::bot::providers::AudioStream;

use super::super::bot::Error;
use super::Regexp;
use super::Search;
use super::SearchResult;

use regex::Regex;
use reqwest::Client as HttpClient;
use serenity::async_trait;
use songbird::input::Input;
use songbird::input::YoutubeDl;

const YOUTUBE_REGEX: &'static str =
    r"(?:https?://)?(?:www\.)?(?:youtube\.com/watch\?v=|youtu\.be/)([a-zA-Z0-9_-]{11})";

const SEARCH_RESULT_LIMIT: usize = 5;

pub struct YouTubeProvider {}

impl AudioStream for YouTubeProvider {
    fn get_stream(&self, http_client: HttpClient, url: String) -> Input {
        let request = YoutubeDl::new(http_client, url);
        return Input::from(request);
    }
}

#[async_trait]
impl Search for YouTubeProvider {
    async fn search(
        &self,
        http_client: HttpClient,
        query: &str,
    ) -> Result<Vec<SearchResult>, Error> {
        let mut request = YoutubeDl::new_search(http_client, query);

        // yt-dlp can return entries without a URL, and we cannot play those.
        let results = request
            .search(Some(SEARCH_RESULT_LIMIT))
            .await?
            .filter_map(|metadata| {
                Some(SearchResult {
                    title: metadata.title.unwrap_or_else(|| "Unknown title".into()),
                    url: metadata.source_url?,
                })
            })
            .collect();

        Ok(results)
    }
}

impl Regexp for YouTubeProvider {
    fn is_valid(&self, input: &str) -> bool {
        return Regex::new(YOUTUBE_REGEX)
            .expect("Youtube regex failed to compile")
            .is_match(input);
    }
}
