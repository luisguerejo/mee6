use crate::bot::{BotContext, DriverStatus, Error, TrackEventHandler};
use std::sync::Arc;

use serenity::{
    builder::{CreateMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption},
    gateway::ActivityData,
    model::{
        application::ComponentInteractionDataKind,
        mention::Mentionable,
        user::{
            OnlineStatus::{Idle, Online},
            User,
        },
    },
};
use songbird::{
    input::{Input, YoutubeDl},
    Event, TrackEvent,
};

use tracing::Level;
use tracing::{error, event, info, warn};

#[poise::command(prefix_command, user_cooldown = 10, aliases("check", "ustraight"))]
pub async fn ping(ctx: BotContext<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[poise::command(prefix_command, user_cooldown = 10, owners_only, aliases("forgive"))]
pub async fn pardon(ctx: BotContext<'_>, arg: User) -> Result<(), Error> {
    ctx.msg.react(&ctx.http(), '👀').await?;
    let mut set = ctx.data().ignoreList.write().await;
    set.remove(&arg);
    Ok(())
}

#[poise::command(prefix_command, user_cooldown = 10, owners_only)]
pub async fn ignore(ctx: BotContext<'_>, arg: User) -> Result<(), Error> {
    ctx.msg.react(&ctx.http(), '👀').await?;
    let mut set = ctx.data().ignoreList.write().await;
    set.insert(arg);
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn skip(ctx: BotContext<'_>) -> Result<(), Error> {
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
pub async fn join(ctx: BotContext<'_>) -> Result<(), Error> {
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
                        let handle = manager.play_input(song.input);
                        let mut driver = status.write().await;
                        *driver = DriverStatus::Playing;
                    }
                }
            });
            ctx.msg.react(&ctx.http(), '👀').await?;
            Ok(())
        }
        Err(e) => {
            error!("Could not join voice channel: {:?}", e);
            panic!("Could not join: {:?}", e)
        }
    }
}

#[poise::command(prefix_command)]
pub async fn leave(ctx: BotContext<'_>) -> Result<(), Error> {
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
pub async fn play(ctx: BotContext<'_>, #[rest] arg: String) -> Result<(), Error> {
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
            let yt = YoutubeDl::new(ctx.data.httpClient.clone(), arg.clone())
                .user_args(vec![String::from("--cookies-from-browser firefox")]);
            let mut status = ctx.data.driverStatus.write().await;
            let input = Input::from(yt);

            match *status {
                DriverStatus::Idle => {
                    let mut vec = ctx.data.queue.lock().await;
                    vec.push_front(input);
                    ctx.data.notify.notify_waiters();
                    *status = DriverStatus::Playing;
                }
                DriverStatus::Playing => {
                    let mut vec = ctx.data.queue.lock().await;
                    vec.push_back(input);
                }
                DriverStatus::Disconnected => {
                    error!("Undefined behavior, should be not allowed to queue songs since Bot is not connected");
                    panic!("Driver is not connected. Should not be queueing songs!")
                }
            }
        }
        false => {
            // Youtube search for the song
            let mut search = YoutubeDl::new_search(ctx.data.httpClient.clone(), arg);
            let results = search
                .search(Some(5))
                .await
                .expect("No query results returned");
            // Format the message to look nicely
            let mut message = String::new();
            for (song, num) in results.iter().zip(1..=5) {
                let title: &String = song.title.as_ref().expect("Should be a song title");
                message.push_str(format!("{num}. {title}\n").as_str())
            }
            let msg = ctx
                .msg
                .channel_id
                .send_message(
                    &ctx,
                    CreateMessage::new().content(message).select_menu(
                        CreateSelectMenu::new(
                            "animal_select",
                            CreateSelectMenuKind::String {
                                options: vec![
                                    CreateSelectMenuOption::new("1️⃣", "1"),
                                    CreateSelectMenuOption::new("2️⃣", "2"),
                                    CreateSelectMenuOption::new("3️⃣", "3"),
                                    CreateSelectMenuOption::new("4️⃣", "4"),
                                    CreateSelectMenuOption::new("5️⃣", "5"),
                                ],
                            },
                        )
                        .custom_id("animal_select")
                        .placeholder("Waiting for selection"),
                    ),
                )
                .await
                .unwrap();
            // The interaction waits for a response on the song message
            let interaction = match msg
                .await_component_interaction(&ctx.serenity_context().shard)
                .timeout(std::time::Duration::from_secs(60))
                .await
            {
                Some(x) => x,
                None => {
                    msg.reply(&ctx, "Timed out").await.unwrap();
                    return Ok(());
                }
            };

            // Fetch what the user selected
            let song = match &interaction.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => &values[0],
                _ => panic!("Unexpected interaction data kind"),
            };

            // Once selected, delete the selection menu so it doesn't get confused
            msg.delete(&ctx).await?;

            let n = match song.as_str() {
                "1" => 0,
                "2" => 1,
                "3" => 2,
                "4" => 3,
                "5" => 4,
                _ => panic!("Bad song selection should not have happened!"),
            };

            let mut status = ctx.data.driverStatus.write().await;
            let song = results.get(n).expect("Should be able to access array");
            let input = YoutubeDl::new(
                ctx.data().httpClient.clone(),
                song.source_url.as_ref().expect("Should be a URL").into(),
            );

            match *status {
                DriverStatus::Idle => {
                    let mut vec = ctx.data.queue.lock().await;
                    vec.push_front(input.into());
                    ctx.data.notify.notify_waiters();
                    *status = DriverStatus::Playing;
                }
                DriverStatus::Playing => {
                    let mut vec = ctx.data.queue.lock().await;
                    vec.push_back(input.into());
                }
                DriverStatus::Disconnected => {
                    error!("Undefined behavior, should be not allowed to queue songs since Bot is not connected");
                    panic!("Driver is not connected. Should not be queueing songs!")
                }
            }

            dbg!(song);
        }
    }

    ctx.msg.react(&ctx.http(), '👀').await?;
    Ok(())
}
