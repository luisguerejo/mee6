use std::env;
use serenity::{
    gateway::ActivityData,
    model::{mention::Mentionable,
    user::OnlineStatus::{Idle, Online}},
    prelude::{Client, GatewayIntents, TypeMapKey}
};
use songbird::input::YoutubeDl;
use songbird::SerenityInit;
use youtube::SongMessage;
use regex::Regex;
use reqwest::Client as HttpClient;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use std::sync::Arc;

mod youtube;
type Error = Box<dyn std::error::Error + Send + Sync>;
type BotContext<'a> = poise::Context<'a, Bot, Error>;

#[allow(non_snake_case)]
struct Bot{
    httpClient: HttpClient,
    youtubeRegex: Regex,
    sender: mpsc::Sender<SongMessage>,
    recvr: Arc<Mutex<mpsc::Receiver<SongMessage>>>
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

    if let None = channel_id {
        let _ = ctx.say(format!("{} You're not in a channel!", ctx.author().mention())).await;
        return Ok(())
    };

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird voice client err")
        .clone();

    match manager.join(guild_id, channel_id.unwrap()).await {
        Ok(manager) => {
            let mut manager_lock = manager.lock().await;
            if let Err(why) = manager_lock.deafen(true).await{
                eprintln!("Could not deafen when joining: {:?}", why);
            }
            let activity = ActivityData::custom("Bumping tunes");
            ctx.serenity_context().set_presence(Some(activity), Online);

            // Setup of consumer thread
            let recvr = Arc::clone(&ctx.data().recvr);
            tokio::spawn(async move {
                let mut receiver = recvr.lock().await;
                while let Some(msg) = receiver.recv().await { // Loop on every single message
                    println!("Received message: {:?}", msg.link);

                }
            });
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
    // Queue's up songs to be played
    // TODO if bot hasn't joined, join channel
    match get_regex(&ctx).await.is_match(&arg){
        true => {
            let author = ctx.author().id;
            let song = YoutubeDl::new(get_http_client(&ctx).await, arg.clone());
            let msg = SongMessage{link: song, from: author};

            if let Err(why) = ctx.data().sender.send(msg).await{
                eprintln!("Error sending {arg} to receiver");
            }
            return Ok(())
        
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

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    println!("Starting application");

    let token = env::var("DISCORD_TOKEN").expect("Expected discord token to be set in environment");

    // Priveleges for the bot
    let intents = GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT;

    // Channels to communicate between threads
    // Consumer thread -> Queue up songs for the voice client thread.
    // Producer threads -> Command handling threads (Sent in a song request)
    let (send, rcv) = mpsc::channel(25);
    let data = Bot{
        httpClient: HttpClient::new(),
        youtubeRegex: Regex::new(r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$").expect("error creating regex"),
        sender: send,
        recvr: Arc::new(Mutex::new(rcv))
    };


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
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                // Initial activity for the bot and register the commands
                let activity = ActivityData::custom("Mimis");
                ctx.set_presence(Some(activity), Idle);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(data)
            })
        })
        .build();

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error creating client");

    client.start().await.expect("Could not start client");
}
