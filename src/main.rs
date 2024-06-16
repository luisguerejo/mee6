use std::env;
use serenity::{
    gateway::ActivityData,
    model::{mention::Mentionable,
    user::OnlineStatus::{Idle, Online}},
    prelude::{Client, GatewayIntents}
};
use songbird::input::YoutubeDl;
use songbird::SerenityInit;
use songbird::input::Input;
use regex::Regex;
use reqwest::Client as HttpClient;
use std::sync::Arc;
use bot::Bot;

mod youtube;
mod bot;
type Error = Box<dyn std::error::Error + Send + Sync>;
type BotContext<'a> = poise::Context<'a, Bot, Error>;


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
            let activity = ActivityData::custom("Bumping tunes");
            ctx.serenity_context().set_presence(Some(activity), Online);

            // Setup of consumer task
            let binding = Arc::clone(&ctx.data().reciever);
            let queue = Arc::clone(&ctx.data().queue);
            let notify = Arc::clone(&ctx.data().notify);
            tokio::spawn(async move {
                let mut rec = binding.lock().await;
                println!("Notifier thread going into loop");
                loop{
                    if let Some(msg) = rec.recv().await {
                        let mut q = queue.lock().await;
                        q.push_back(msg);
                        notify.notify_waiters();
                    }
                }
            });

            let manager_handle = Arc::clone(&manager);
            let notify = Arc::clone(&ctx.data().notify);
            let queue = Arc::clone(&ctx.data().queue);
            tokio::spawn(async move {
                loop {
                    notify.notified().await;
                    let mut queue = queue.lock().await;
                    if let Some(song) = queue.pop_front(){
                        let mut manager = manager_handle.lock().await;
                        manager.play_input(song.input);
                    }
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
            let yt = YoutubeDl::new(get_http_client(&ctx).await, arg.clone());
            let msg = youtube::SongMessage{link: arg.clone(), input: Input::from(yt), from: author};

            if let Ok(result) = ctx.data().sender.send(msg){
                println!("from fn play: Sent message and notifed waiters!: {:?}", result);
            }else{
                println!("from fn play: Error sending message!");
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

async fn get_http_client(ctx: &BotContext<'_>) -> HttpClient{
    ctx.data().httpClient.clone()
}

#[tokio::main(flavor = "multi_thread", worker_threads=8)]
async fn main() {
    println!("Starting application");

    let token = env::var("DISCORD_TOKEN").expect("Expected discord token to be set in environment");

    // Priveleges for the bot
    let intents = GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Channels to communicate between threads
    // Consumer thread -> Queue up songs for the voice client thread.
    // Producer threads -> Command handling threads (Sent in a song request)
    let data = Bot::new();


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
