use super::commands;
use super::driver::Driver;
use super::regex::Regexp;
use poise::structs::Command;
use reqwest::Client as HttpClient;
use songbird::input::{Input, YoutubeDl};

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
        if Regexp::is_supported_link(user_input.as_str()) {
            let query = YoutubeDl::new(self.http_client.clone(), user_input);
            let input = Input::from(query);
            self.driver.enqueue_input(input).await?
        }

        Ok(())
    }
}
