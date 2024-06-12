use std::env;

use serenity::prelude::GatewayIntents;
use serenity::prelude::Client;
use serenity::prelude::Context;
use serenity::model::channel::Message;
use poise::serenity_prelude as serenity;
use songbird::SerenityInit;
use std::collections::vec_deque::VecDeque;

mod youtube;
type Error = Box<dyn std::error::Error + Send + Sync>;
type BotContext<'a> = poise::Context<'a, VecDeque<youtube::SongMessage>, Error>;

#[poise::command(prefix_command)]
async fn ping(ctx: BotContext<'_>) -> Result<(), Error>{
    ctx.say("Pong!").await?;
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

    let framework = poise::Framework::<VecDeque<youtube::SongMessage>, Error>::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ping()],
            skip_checks_for_owners: true,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                case_insensitive_commands: true,
                edit_tracker: None,
                execute_self_messages: false,
                ..Default::default()
            },
            ..Default::default()
        }).setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                ctx.set_presence(None, serenity::model::user::OnlineStatus::Idle);
                let commands = poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(VecDeque::<youtube::SongMessage>::new())
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
