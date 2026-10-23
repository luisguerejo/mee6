mod soundcloud;
mod youtube;

use super::bot::Error;

use crate::bot::providers::soundcloud::SoundCloudProvider;
use crate::bot::providers::youtube::YouTubeProvider;

use reqwest::Client as HttpClient;
use serenity::async_trait;
use songbird::input::Input;

pub enum Providers {
    YouTube(YouTubeProvider),
    SoundCloud(SoundCloudProvider),
}

impl Providers {
    pub fn all() -> Vec<Providers> {
        vec![
            Providers::YouTube(YouTubeProvider {}),
            Providers::SoundCloud(SoundCloudProvider {}),
        ]
    }

    pub fn get_stream(&self, http_client: HttpClient, url: String) -> Input {
        match self {
            Providers::YouTube(p) => p.get_stream(http_client, url),
            Providers::SoundCloud(p) => p.get_stream(http_client, url),
        }
    }

    pub async fn search(
        &self,
        http_client: HttpClient,
        query: &str,
    ) -> Result<Vec<SearchResult>, Error> {
        match self {
            Providers::YouTube(p) => p.search(http_client, query).await,
            Providers::SoundCloud(p) => p.search(http_client, query).await,
        }
    }

    pub fn is_valid(&self, input: &str) -> bool {
        match self {
            Providers::YouTube(p) => p.is_valid(input),
            Providers::SoundCloud(p) => p.is_valid(input),
        }
    }
}

pub trait AudioStream {
    fn get_stream(&self, http_client: HttpClient, url: String) -> Input;
}

pub struct SearchResult {
    pub title: String,
    pub url: String,
}

#[async_trait]
pub trait Search: Send + Sync {
    async fn search(&self, http_client: HttpClient, query: &str)
        -> Result<Vec<SearchResult>, Error>;
}

pub trait Regexp {
    fn is_valid(&self, input: &str) -> bool;
}
