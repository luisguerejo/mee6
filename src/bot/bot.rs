use reqwest::Client as HttpClient;
use poise::structs::Command;
use super::driver::Driver;
use super::commands;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::PrefixContext<'a, Bot, Error>;

#[allow(non_snake_case)]
pub struct Bot {
    pub http_client: HttpClient,
    pub driver: Driver,
}

impl Bot {
    pub fn new() -> Self {
        Self{
            http_client: HttpClient::new(),
            driver: Driver::new(),
        }
    }

    pub fn commands(&self) -> Vec<Command<Bot, Error>> {
        vec![
            commands::ping(),
            commands::play(),
            commands::pause(),
            commands::join(),
            commands::leave(),
        ]
    }
}
