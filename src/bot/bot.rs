use crate::bot::providers::{Providers, SearchResult};

use super::commands;
use super::driver::Driver;
use poise::structs::Command;
use reqwest::Client as HttpClient;
use tracing::warn;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::PrefixContext<'a, Bot, Error>;

pub struct Bot {
    pub http_client: HttpClient,
    pub driver: Driver,
}

impl Bot {
    pub fn new() -> Self {
        Self {
            http_client: HttpClient::new(),
            driver: Driver::new(),
        }
    }

    pub fn commands() -> Vec<Command<Bot, Error>> {
        vec![
            commands::ping(),
            commands::play(),
            commands::pause(),
            commands::skip(),
            commands::join(),
            commands::leave(),
        ]
    }

    pub fn is_link(&self, user_input: &str) -> bool {
        Providers::all().iter().any(|p| p.is_valid(user_input))
    }

    /// Returns the results of the first provider that finds any. An empty
    /// list means nothing was found, not that the search failed.
    pub async fn search(&self, query: &str) -> Vec<SearchResult> {
        for provider in Providers::all() {
            // One broken provider (or one not implemented yet) must not stop
            // the others from being tried.
            let results = provider
                .search(self.http_client.clone(), query)
                .await
                .unwrap_or_else(|e| {
                    warn!("Search failed: {e}");
                    Vec::new()
                });

            if !results.is_empty() {
                return results;
            }
        }

        Vec::new()
    }

    pub async fn play_input(&self, url: String) -> Result<(), Error> {
        let providers = Providers::all();
        let provider = providers
            .iter()
            .find(|p| p.is_valid(&url))
            .ok_or("No valid stream found")?;

        let stream = provider.get_stream(self.http_client.clone(), url);
        self.driver.enqueue_input(stream).await
    }
}
