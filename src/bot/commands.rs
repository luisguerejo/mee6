use super::bot::{Context, Error};

use serenity::model::mention::Mentionable;

use tracing::info;

#[poise::command(prefix_command, user_cooldown = 10, aliases("check", "ustraight"))]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    info!("!SKIP invoked by {:?}", &ctx.author().name,);
    ctx.data.driver.skip_current_track().await
}

#[poise::command(prefix_command)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    info!("PAUSE invoked by {:?}", &ctx.author().name);
    ctx.data.driver.pause_current_track().await
}

#[poise::command(prefix_command)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    info!("!JOIN by {:?}", &ctx.author().name,);

    let (guild_id, channel_id) = {
        let guild = ctx.guild().unwrap();
        let channel_id = guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };

    if channel_id.is_none() {
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

    let call = manager
        .join(guild_id, channel_id.unwrap())
        .await
        .expect("Could not get discord call");

    let driver = ctx.data().driver.clone();

    tokio::spawn(async move {
        driver.player(call).await;
    });

    ctx.msg.react(&ctx.http(), '👀').await?;
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    info!("LEAVE invoked by {:?}", &ctx.author().name,);

    let guild_id = ctx.msg.guild(&ctx.cache()).unwrap().id;
    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Could not get songbird client")
        .clone();

    ctx.data.driver.leave(manager, guild_id).await
}

#[poise::command(prefix_command, aliases("p", "queue", "q"))]
pub async fn play(ctx: Context<'_>, #[rest] argument: Option<String>) -> Result<(), Error> {
    info!("PLAY invoked by {:?}", &ctx.author().name);

    if argument.is_none() {
        return ctx.data.driver.unpause_current_track().await;
    }

    ctx.data.play_input(argument.unwrap()).await?;
    ctx.msg.react(&ctx.http(), '✅').await?;
    Ok(())
}
