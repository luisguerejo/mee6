use std::env;

use songbird::SerenityInit;
use songbird::events::{Event, EventHandler as VoiceEventHandler, TrackEvent, EventContext};
use serenity::all::{GatewayIntents};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use serenity::client::Context;

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
            "!join" => {
                msg.react(&ctx.http, '👀').await;
                let (guild_id, channel_id) = {
                    let guild = msg.guild(&ctx.cache).unwrap();
                    let channel_id = guild
                        .voice_states
                        .get(&msg.author.id)
                        .and_then(|voice_state| voice_state.channel_id);

                    (guild.id, channel_id)
                };
                println!("Guild ID: {:?}\n Channel ID: {:?}", guild_id, channel_id);

                let connect = match channel_id {
                    Some(channel) => channel,
                    None => {
                        panic!("Could not match channel_id!");
                    }
                };

                let manager = songbird::get(&ctx)
                    .await
                    .expect("Songbird Voice client placed")
                    .clone();




                match  manager.join(guild_id, connect).await {
                    Ok(_) => {
                        println!("Succesfully connected!");
                    },
                    Err(e) => panic!("Could not join: {:?}", e)
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

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .register_songbird()
        .await.expect("Error creating client");

    client.start().await;
}
