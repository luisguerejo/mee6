use crate::bot::providers::Providers;

use super::commands;
use super::driver::Driver;
use poise::structs::Command;
use reqwest::Client as HttpClient;
use songbird::input::Input;

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

    pub async fn play_input(&self, user_input: String) -> Result<(), Error> {
        let providers = Providers::all();
        let mut stream: Option<Input> = None;

        if let Some(provider) = providers.iter().find(|p| p.is_valid(&user_input)) {
            stream = Some(provider.get_stream(self.http_client.clone(), user_input));
        } else {
            for provider in providers {
                let results = provider.search(&user_input).await?;
                if let Some(result) = results.get(0) {
                    stream = Some(provider.get_stream(self.http_client.clone(), result.clone()));
                    break;
                }
            }
        }

        if let Some(stream) = stream {
            self.driver.enqueue_input(stream).await?;
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No valid stream found",
            )));
        }
        Ok(())
    }
}
