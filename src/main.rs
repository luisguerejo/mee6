use std::env;

use serenity::all::GatewayIntents;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::prelude::*;
use songbird::SerenityInit;

use poise::serenity_prelude;
type Error = &'static str;
type BotFramework<'a> = poise::Framework<Context, Error>;
struct Handler;
struct General;

async fn ping(_ctx: Context, _msg: Message) -> Result<(), SerenityError> {
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

    let framework = poise::Framework::builder()
        .setup(|ctx: &'_ Context, _, framework: &BotFramework| {
            Box::pin(async move {
                ctx.set_presence(None, serenity::model::user::OnlineStatus::Idle);
                let commands = poise::builtins::create_application_commands(&framework.options().commands);
                Ok(ctx)
            })
        })
        .options(poise::FrameworkOptions {
            skip_checks_for_owners: true,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                case_insensitive_commands: true,
                edit_tracker: None,
                execute_self_messages: false,
                ..Default::default()
            },
            ..Default::default()
        })
        .build();
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error creating client");

    client.start().await;
}
