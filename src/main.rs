use std::env;

use serenity::all::CacheHttp;
use serenity::all::Http;
use serenity::prelude::GatewayIntents;
use serenity::prelude::Client;
use serenity::model::user::OnlineStatus::{Idle, Online};
use serenity::gateway::ActivityData;
use songbird::input::YoutubeDl;
use songbird::SerenityInit;
use youtube::SongMessage;
use std::collections::vec_deque::VecDeque;
use regex::Regex;
use reqwest::Client as HttpClient;

mod youtube;
type Error = Box<dyn std::error::Error + Send + Sync>;
type BotContext<'a> = poise::Context<'a, Bot, Error>;

struct Bot{
    pub httpClient: HttpClient,
    youtubeRegex: Regex,
    songQueue: VecDeque<youtube::SongMessage>
}

#[poise::command(
    prefix_command,
    user_cooldown=10,
    aliases("check", "ustraight")
)]
async fn ping(ctx: BotContext<'_>) -> Result<(), Error>{
    ctx.say("Pong!").await?;
    Ok(())
}

#[poise::command(prefix_command)]
async fn join(
    ctx: BotContext<'_>)
    -> Result<(), Error>{
    let (guild_id, channel_id) = {
        let guild = ctx.guild().unwrap();
        let channel_id = guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };

    let connect = match channel_id {
        Some(channel) => channel,
        None => {
            panic!("Could not match channel_id!")
        }
    };

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird voice client err")
        .clone();

    match manager.join(guild_id, connect).await {
        Ok(_) => {
            let activity = ActivityData::custom("Bumping tunes");
            ctx.serenity_context().set_presence(Some(activity), Online);
            Ok(())
        },
        Err(e) => panic!("Could not join: {:?}", e)
    }
}

#[poise::command(
    prefix_command,
    aliases("p", "queue", "q")
)]
async fn play(ctx: BotContext<'_>, #[rest] arg: String) -> Result<(), Error>{
    let regex = Regex::new(r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$")?;
    match regex.is_match(&arg){
        true => {
            ctx.say("Got a link!").await?;
        },
        false => {
            ctx.say("Did not get a link!").await?;
        }
    };

    Ok(())
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

    let framework = poise::Framework::<Bot, Error>::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                ping(),
                join(),
                play()
            ],
            skip_checks_for_owners: true,
            manual_cooldowns: false,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                edit_tracker: None,
                execute_self_messages: false,
                ignore_thread_creation: true,
                ..Default::default()
            },
            ..Default::default()
        }).setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                let activity = ActivityData::custom("Mimis");
                ctx.set_presence(Some(activity), Idle);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Bot{
                    httpClient: HttpClient::new(),
                    youtubeRegex: Regex::new(r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$").expect("error creating regex"),
                    songQueue: VecDeque::new()
                })
            })
        })
        .build();

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error creating client");

    client.start().await;
}