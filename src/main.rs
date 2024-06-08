use std::env;

use serenity::all::GatewayIntents;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        match msg.content.as_str() {
            "!ping" => {
                msg.react(&ctx.http, '👀').await;
                msg.channel_id.say(&ctx.http, "Pong!").await;
            },
            "!deleteme" => {
                eprintln!("Got a delete me");
                let result = msg.delete(&ctx.http).await;
                if result.is_err(){
                    eprintln!("Error deleting message");
                    eprintln!("Error msg: {:?}", result.unwrap());
                }
            },
            _ => ()
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Starting application");
    
    let token = env::var("DISCORD_TOKEN").expect("Expected discord token to be set in environment");

    let intents = GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents).event_handler(Handler).await.expect("Error creating client");

    client.start().await;

}
