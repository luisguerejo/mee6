use std::env;

use serenity::prelude::GatewayIntents;
use serenity::prelude::Client;
use serenity::model::channel::Message;
use songbird::SerenityInit;
use std::collections::vec_deque::VecDeque;

mod youtube;
type Error = Box<dyn std::error::Error + Send + Sync>;
type BotContext<'a> = poise::Context<'a, VecDeque<youtube::SongMessage>, Error>;

#[poise::command(
    prefix_command,
    user_cooldown=10,
    aliases("check", "ustraight")
)]
async fn ping(ctx: BotContext<'_>) -> Result<(), Error>{
    ctx.say("Pong!").await?;
    Ok(())
}

#[poise::command(prefix_command, aliases("link"))]
async fn join(
    ctx: BotContext<'_>,
    msg: Message) -> Result<(), Error>{
    ctx.say("Joining!").await?;
    let (guild_id, channel_id) = {
        let guild = msg.guild(&ctx.cache()).unwrap();
        let channel_id = guild
            .voice_states
            .get(&msg.author.id)
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
            eprintln!("Succesfully connected!");
        }
        Err(e) => panic!("Could not join: {:?}", e)
    }
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
            commands: vec![
                ping(),
                join()
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
                ctx.set_presence(None, serenity::model::user::OnlineStatus::Idle);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
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
