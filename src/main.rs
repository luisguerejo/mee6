use std::env;
use serenity::{
    prelude::{GatewayIntents, Client, TypeMapKey},
    model::user::OnlineStatus::{Idle, Online},
    gateway::ActivityData
};
use songbird::input::YoutubeDl;
use songbird::SerenityInit;
use youtube::SongMessage;
use std::collections::vec_deque::VecDeque;
use regex::Regex;
use reqwest::Client as HttpClient;

mod youtube;
type Error = Box<dyn std::error::Error + Send + Sync>;
type BotContext<'a> = poise::Context<'a, Bot, Error>;

#[allow(non_snake_case)]
struct Bot{
    httpClient: HttpClient,
    youtubeRegex: Regex,
    songQueue: VecDeque<SongMessage>
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

impl TypeMapKey for Bot{
    type Value = Bot;
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
async fn play(ctx: BotContext<'_>, #[rest] song: String) -> Result<(), Error>{
    // Queue's up songs to be played
    // TODO if bot hasn't joined, join channel
    match get_regex(&ctx).await.is_match(&song){
        true => {
            let author = ctx.author().id;
            let song = YoutubeDl::new(get_http_client(&ctx).await, song);
            let guild = ctx.guild().unwrap();
            let msg = SongMessage{link: song, from: author};
        
        },
        false => {
            ctx.say("Did not get a link!").await?;
        }
    };

    Ok(())
}

async fn get_regex(ctx: &BotContext<'_>) -> Regex{
    ctx.data().youtubeRegex.clone()
}

async fn get_http_client(ctx: &BotContext<'_>) -> reqwest::Client{
    ctx.data().httpClient.clone()
}

#[tokio::main]
async fn main() {
    println!("Starting application");

    let token = env::var("DISCORD_TOKEN").expect("Expected discord token to be set in environment");

    // Priveleges for the bot
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
        .type_map_insert::<Bot>(Bot{
                    httpClient: HttpClient::new(),
                    youtubeRegex: Regex::new(r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$").expect("error creating regex"),
                    songQueue: VecDeque::new()
            })
        .await
        .expect("Error creating client");

    client.start().await.expect("Could not start client");
}