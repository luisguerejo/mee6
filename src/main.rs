use bot::{Bot, DriverStatus};
use serenity::{
    async_trait,
    gateway::ActivityData,
    model::{
        id::UserId,
        mention::Mentionable,
        user::OnlineStatus::{Idle, Online},
        user::User,
    },
    prelude::{Client, GatewayIntents},
};
use songbird::{
    input::{Input, YoutubeDl},
    Event, EventContext, EventHandler as VoiceEventHandler, SerenityInit, TrackEvent,
};
use std::{
    collections::{HashSet, VecDeque},
    env,
    sync::Arc,
};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, event, info, warn, Level};

mod bot;
mod youtube;
type Error = Box<dyn std::error::Error + Send + Sync>;
type BotContext<'a> = poise::PrefixContext<'a, Bot, Error>;

struct TrackEventHandler {
    notify: Arc<tokio::sync::Notify>,
    queue: Arc<Mutex<VecDeque<youtube::SongMessage>>>,
    driver: Arc<RwLock<DriverStatus>>,
}

#[async_trait]
impl VoiceEventHandler for TrackEventHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let queue = self.queue.lock().await;
        if !queue.front().is_none() {
            self.notify.notify_waiters();
        } else {
            let mut driver = self.driver.write().await;
            *driver = DriverStatus::Idle;
        }
        return None;
    }
}

#[poise::command(prefix_command, user_cooldown = 10, aliases("check", "ustraight"))]
async fn ping(ctx: BotContext<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[poise::command(prefix_command, user_cooldown = 10, owners_only, aliases("forgive"))]
async fn pardon(ctx: BotContext<'_>, arg: User) -> Result<(), Error> {
    ctx.msg.react(&ctx.http(), '👀').await?;
    let mut set = ctx.data().ignoreList.write().await;
    set.remove(&arg);
    Ok(())
}

#[poise::command(prefix_command, user_cooldown = 10, owners_only)]
async fn ignore(ctx: BotContext<'_>, arg: User) -> Result<(), Error> {
    ctx.msg.react(&ctx.http(), '👀').await?;
    let mut set = ctx.data().ignoreList.write().await;
    set.insert(arg);
    Ok(())
}

#[poise::command(prefix_command)]
async fn skip(ctx: BotContext<'_>) -> Result<(), Error> {
    info!(
        "SKIP invoked by {:?}:{:?}\n DriverStatus: {:?}",
        &ctx.author().name,
        &ctx.msg.content,
        ctx.data().driverStatus
    );
    {
        // Check if user is being ignored
        let set = &ctx.data.ignoreList;
        let set = set.read().await;
        if set.contains(ctx.author()) {
            ctx.msg.react(ctx.http(), '💀').await?;
            return Ok(());
        }
    }
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Error getting Songbird client")
        .clone();

    match manager.get(ctx.guild_id().unwrap()) {
        Some(connection) => {
            let mut call = connection.lock().await;
            let notify = Arc::clone(&ctx.data().notify);
            let mut status = ctx.data().driverStatus.write().await;
            let song_queue = Arc::clone(&ctx.data().queue);

            match *status {
                DriverStatus::Playing => {
                    call.stop();
                    match song_queue.lock().await.front() {
                        Some(_) => {
                            notify.notify_waiters();
                        }
                        None => *status = DriverStatus::Idle,
                    }
                }
                DriverStatus::Idle => {
                    let author = &ctx.author().name;
                    let msg = &ctx.msg.content;
                    event!(
                        Level::ERROR,
                        "{:?}:{:?} Tried skipping when DriverStatus is idle",
                        author,
                        msg
                    );
                }
                DriverStatus::Disconnected => {
                    error!("Undefined behavior. Should not be able to get a connection");
                    panic!("Undefined behavior. Should not be able to get a connection")
                }
            }
        }
        None => eprintln!("Could not get connection to skip!"),
    }

    Ok(())
}

#[poise::command(prefix_command)]
async fn join(ctx: BotContext<'_>) -> Result<(), Error> {
    info!(
        "JOIN invoked by {:?}:{:?}\n DriverStatus: {:?}",
        &ctx.author().name,
        &ctx.msg.content,
        ctx.data().driverStatus
    );
    let (guild_id, channel_id) = {
        let guild = ctx.guild().unwrap();
        let channel_id = guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };

    if channel_id.is_none() {
        warn!(
            "{:?} is not a voice channel! Cannot connect to voice channel",
            &ctx.author().name
        );
        ctx.say(format!(
            "{} You're not in a channel!",
            ctx.author().mention()
        ))
        .await?;
        return Ok(());
    }

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird voice client err")
        .clone();

    match manager.join(guild_id, channel_id.unwrap()).await {
        Ok(manager) => {
            let activity = ActivityData::custom("bumpin' tunes");
            ctx.serenity_context().set_presence(Some(activity), Online);
            {
                let mut status = ctx.data().driverStatus.write().await;
                *status = DriverStatus::Idle;
            }

            let mut handle = manager.lock().await;
            handle.add_global_event(
                Event::Track(TrackEvent::End),
                TrackEventHandler {
                    notify: ctx.data().notify.clone(),
                    queue: Arc::clone(&ctx.data.queue),
                    driver: Arc::clone(&ctx.data().driverStatus),
                },
            );

            let manager_handle = Arc::clone(&manager);
            let notify = Arc::clone(&ctx.data().notify);
            let queue = Arc::clone(&ctx.data().queue);
            let status = Arc::clone(&ctx.data().driverStatus);
            tokio::spawn(async move {
                loop {
                    notify.notified().await;
                    let mut queue = queue.lock().await;
                    if let Some(song) = queue.pop_front() {
                        let mut manager = manager_handle.lock().await;
                        manager.play_input(song.input);
                        let mut driver = status.write().await;
                        *driver = DriverStatus::Playing;
                    }
                }
            });
            Ok(())
        }
        Err(e) => {
            error!("Could not join voice channel: {:?}", e);
            panic!("Could not join: {:?}", e)
        }
    }
}

#[poise::command(prefix_command)]
async fn leave(ctx: BotContext<'_>) -> Result<(), Error> {
    info!(
        "LEAVE invoked by {:?}:{:?}\n DriverStatus: {:?}",
        &ctx.author().name,
        &ctx.msg.content,
        ctx.data().driverStatus
    );
    let guild_id = ctx.msg.guild(&ctx.cache()).unwrap().id;

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Could not get songbird client")
        .clone();

    let handler = manager.get(guild_id).is_some();

    if handler {
        if let Err(e) = manager.remove(guild_id).await {
            error!("Error leaving voice channel: {:?}", e);
        }
        let activity = ActivityData::custom("mimis");
        ctx.serenity_context().set_presence(Some(activity), Idle);

        let mut bot_status = ctx.data.driverStatus.write().await;
        *bot_status = DriverStatus::Disconnected;

        let mut queue = ctx.data.queue.lock().await;
        queue.clear();
    }

    Ok(())
}

#[poise::command(prefix_command, aliases("p", "queue", "q"))]
async fn play(ctx: BotContext<'_>, #[rest] arg: String) -> Result<(), Error> {
    info!(
        "PLAY invoked by {:?}:{:?}\n DriverStatus: {:?}",
        &ctx.author().name,
        &ctx.msg.content,
        ctx.data().driverStatus
    );
    // Queue's up songs to be played
    {
        // Check if user is being ignored
        let set = &ctx.data.ignoreList;
        let set = set.read().await;
        if set.contains(ctx.author()) {
            ctx.msg.react(ctx.http(), '💀').await?;
            return Ok(());
        }
    }

    match ctx.data.youtubeRegex.is_match(&arg) {
        true => {
            let author = ctx.author().id;
            let yt = YoutubeDl::new(ctx.data.httpClient.clone(), arg.clone());
            let msg = youtube::SongMessage {
                link: arg.clone(),
                input: Input::from(yt),
                from: author,
            };
            let mut status = ctx.data.driverStatus.write().await;

            match *status {
                DriverStatus::Idle => {
                    let mut vec = ctx.data.queue.lock().await;
                    vec.push_front(msg);
                    ctx.data.notify.notify_waiters();
                    *status = DriverStatus::Playing;
                }
                DriverStatus::Playing => {
                    let mut vec = ctx.data.queue.lock().await;
                    vec.push_back(msg);
                }
                DriverStatus::Disconnected => {
                    error!("Undefined behavior, should be not allowed to queue songs since Bot is not connected");
                    panic!("Driver is not connected. Should not be queueing songs!")
                }
            }
        }
        false => {
            warn!("None link song queuing supported yet");
            ctx.say("Did not get a link!").await?;
        }
    }

    ctx.msg.react(&ctx.http(), '👀').await?;
    Ok(())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    tracing_subscriber::fmt::init();
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
            commands: vec![ping(), join(), play(), ignore(), pardon(), leave(), skip()],
            skip_checks_for_owners: true,
            manual_cooldowns: false,
            owners: HashSet::from([UserId::new(90550255229091840)]),
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
                let activity = ActivityData::custom("mimis");
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
    info!("Bot is starting...")
}
