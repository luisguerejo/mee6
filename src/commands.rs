use crate::bot::{BotContext, DriverStatus, Error, TrackEventHandler};
use std::sync::Arc;

use crate::tarkov::utils::{fetch_task, format_task_response, load_quests};

use serenity::{
    builder::{CreateMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption},
    gateway::ActivityData,
    model::{
        application::ComponentInteractionDataKind,
        mention::Mentionable,
        user::OnlineStatus::{Idle, Online},
    },
};
use songbird::{
    input::{Input, YoutubeDl},
    Event, TrackEvent,
};

use tracing::{error, info, warn};

#[poise::command(prefix_command, user_cooldown = 10, aliases("check", "ustraight"))]
pub async fn ping(ctx: BotContext<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
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

    let current_track = Arc::clone(&ctx.data.currentTrack);
    let mut current_track = current_track.lock().await;

    if let Some(track) = &mut *current_track {
        let status = Arc::clone(&ctx.data.driverStatus);
        let status = status.read().await;
        match *status {
            DriverStatus::Playing => {
                track.stop()?;
                *current_track = None;
            },
            DriverStatus::Paused => ctx.data.notify.notify_one(),
            DriverStatus::Disconnected | DriverStatus::Idle => panic!("Should not be able to reach Disconnected or Idle status if there is no current track!")
        }
    } else {
        ctx.reply("No current song to skip!").await?;
    }
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn pause(ctx: BotContext<'_>) -> Result<(), Error> {
    info!(
        "PAUSE invoked by {:?}:{:?}\n DriverStatus: {:?}",
        &ctx.author().name,
        &ctx.msg.content,
        ctx.data().driverStatus
    );
    let current_track = Arc::clone(&ctx.data().currentTrack);
    let current_track = current_track.lock().await;

    let status = Arc::clone(&ctx.data().driverStatus);
    let mut status = status.write().await;

    match *current_track {
        Some(ref track) => {
            track.pause().expect("Error pausing");
            *status = DriverStatus::Paused;
        }
        None => {
            error!(
                "{:?} tried to pause when there is no current track!",
                ctx.author().name
            );
        }
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
            let current_track = Arc::clone(&ctx.data().currentTrack);
            tokio::spawn(async move {
                loop {
                    notify.notified().await;
                    let mut queue = queue.lock().await;
                    if let Some(song) = queue.pop_front() {
                        // Need to grab all associated locks
                        let mut manager = manager_handle.lock().await;
                        let mut current = current_track.lock().await;

                        // Set current track handle from the result of
                        // play input
                        let track_handle = manager.play_input(song);
                        *current = Some(track_handle);

                        // Atomically change the DriverStatus
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

    let queue = Arc::clone(&ctx.data.queue);
    let mut queue = queue.lock().await;

    let handler = manager.get(guild_id).is_some();

    let current_track = Arc::clone(&ctx.data.currentTrack);
    let mut current_track = current_track.lock().await;

    let bot_status = Arc::clone(&ctx.data.driverStatus);
    let mut bot_status = bot_status.write().await;

    if handler {
        if let Err(e) = manager.leave(guild_id).await {
            error!("Error leaving voice channel: {:?}", e);
        }
        let activity = ActivityData::custom("mimis");
        ctx.serenity_context().set_presence(Some(activity), Idle);

        queue.clear();

        if let Some(ref track) = *current_track {
            if let Err(e) = track.stop() {
                panic!("Error stopping current track when leaving: {e}");
            }
        }
        *current_track = None;

        *bot_status = DriverStatus::Disconnected;
    }

    Ok(())
}

#[poise::command(prefix_command, aliases("p", "queue", "q"))]
pub async fn play(ctx: BotContext<'_>, #[rest] argument: Option<String>) -> Result<(), Error> {
    info!(
        "PLAY invoked by {:?}:{:?}\n DriverStatus: {:?}",
        &ctx.author().name,
        &ctx.msg.content,
        ctx.data.driverStatus
    );
    // Queue's up songs to be played
    let queue = Arc::clone(&ctx.data.queue);
    let mut queue = queue.lock().await;

    let current_track = Arc::clone(&ctx.data.currentTrack);
    let current_track = current_track.lock().await;

    let status = Arc::clone(&ctx.data.driverStatus);
    let mut status = status.write().await;

    if argument.is_none() && *status == DriverStatus::Paused {
        match *current_track {
            Some(ref track) => {
                track
                    .play()
                    .expect("Error resuming track from !play command");
                *status = DriverStatus::Playing;
            }
            None => {
                error!(
                    "!play command invoked by {:?}: No current track to resume!",
                    ctx.author().name
                );
            }
        }
        println!(
            "Resuming from play:\n status: {:?}\n current_track: {:?}\n",
            status, current_track
        );
        return Ok(());
    } else if argument.is_none() {
        error!(
            "PLAY invoked by {:?}:{:?}\n DriverStatus: {:?}",
            &ctx.author().name,
            &ctx.msg.content,
            ctx.data.driverStatus
        );
        return Ok(());
    }

    let arg = argument.unwrap();

    if ctx.data.youtubeRegex.is_match(&arg) || ctx.data.soundcloudRegex.is_match(&arg) {
        let yt = YoutubeDl::new(ctx.data.httpClient.clone(), arg.clone());
        let input = Input::from(yt);

        match *status {
            DriverStatus::Idle => {
                queue.push_front(input);
                ctx.data.notify.notify_one();
                *status = DriverStatus::Playing;
            }
            DriverStatus::Playing | DriverStatus::Paused => {
                queue.push_back(input);
            }
            DriverStatus::Disconnected => {
                error!("Undefined behavior, should be not allowed to queue songs since Bot is not connected");
                panic!("Driver is not connected. Should not be queueing songs!")
            }
        }
        ctx.msg.react(&ctx.http(), '✅').await?;
        return Ok(());
    }
    // Youtube search for the song
    let mut search = YoutubeDl::new_search(ctx.data.httpClient.clone(), arg);
    let results: Vec<_> = search
        .search(Some(5))
        .await
        .expect("No query results returned")
        .collect();
    // Format the message to look nicely
    let mut message = String::new();
    for (song, num) in results.iter().zip(1..=3) {
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
                    "song_select",
                    CreateSelectMenuKind::String {
                        options: vec![
                            CreateSelectMenuOption::new("1️⃣", "1"),
                            CreateSelectMenuOption::new("2️⃣", "2"),
                            CreateSelectMenuOption::new("3️⃣", "3"),
                        ],
                    },
                )
                .custom_id("song_select")
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
            msg.delete(&ctx)
                .await
                .expect("Error deleting song selection menu");
            ctx.msg.reply(&ctx, "Timed out").await?;
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
        _ => panic!("Bad song selection should not have happened!"),
    };

    let song = results.get(n).expect("Should be able to access array").clone();
    let input = YoutubeDl::new(
        ctx.data().httpClient.clone(),
        song.source_url.expect("Error getting selected song URL"),
    );

    match *status {
        DriverStatus::Idle => {
            queue.push_front(input.into());
            ctx.data.notify.notify_one();
            *status = DriverStatus::Playing;
        }
        DriverStatus::Playing | DriverStatus::Paused => {
            queue.push_back(input.into());
        }
        DriverStatus::Disconnected => {
            error!("Undefined behavior, should be not allowed to queue songs since Bot is not connected");
            panic!("Driver is not connected. Should not be queueing songs!")
        }
    }

    ctx.msg.react(&ctx.http(), '✅').await?;
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn quest(
    ctx: BotContext<'_>,
    #[description = "Quest name"] name: String,
) -> Result<(), Error> {
    let quests = match load_quests().await {
        Ok(quests) => quests,
        Err(e) => {
            error!("Failed to load quests: {}", e);
            ctx.say("Failed to load quests. Please try again later.")
                .await?;
            return Ok(());
        }
    };

    let matching_quests: Vec<_> = quests
        .iter()
        .filter(|quest| quest.name.to_lowercase().contains(&name.to_lowercase()))
        .collect();

    match matching_quests.len() {
        0 => {
            ctx.say("No quests found with that name.").await?;
        }
        1 => {
            let quest = &matching_quests[0];

            match fetch_task(&quest.id).await {
                Ok(response) => {
                    let message = format_task_response(&response.data.task);
                    ctx.say(message).await?;
                }
                Err(_) => {
                    ctx.say("Failed to fetch quest details.").await?;
                }
            }
            return Ok(());
        }
        _ => {
            let quest_list = matching_quests
                .iter()
                .map(|quest| quest.name.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            ctx.say(format!("Found multiple matching quests:\n{}", quest_list))
                .await?;
        }
    }

    Ok(())
}

#[poise::command(prefix_command)]
pub async fn debug(ctx: BotContext<'_>) -> Result<(), Error> {
    let current_track = Arc::clone(&ctx.data.currentTrack);
    let current_track = current_track.lock().await;

    let driver_status = Arc::clone(&ctx.data.driverStatus);
    let driver_status = driver_status.read().await;

    let msg = format!(
        r#"Current Track: {:?}
        DriverStatus: {:?}
    "#,
        *current_track, driver_status
    );

    ctx.reply(msg).await?;

    Ok(())
}
