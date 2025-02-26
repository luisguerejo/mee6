use bot::Bot;
use serenity::{
    gateway::ActivityData,
    model::{id::UserId, user::OnlineStatus::Idle},
    prelude::{Client, GatewayIntents},
};
use songbird::SerenityInit;
use std::{collections::HashSet, env};
use tracing::info;

mod commands;

mod bot;
mod tarkov;

use bot::Error;

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
            commands: vec![
                commands::ping(),
                commands::join(),
                commands::play(),
                commands::quest(),
                // commands::ignore(),
                // commands::pardon(),
                commands::leave(),
                commands::skip(),
            ],
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
