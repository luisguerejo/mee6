use super::bot::{Context, Error};
use super::providers::SearchResult;

use serenity::all::{
    ComponentInteractionDataKind, CreateInteractionResponse, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use std::time::Duration;

const MENU_ID: &str = "song_select";
const SELECTION_TIMEOUT: Duration = Duration::from_secs(60);
// Discord rejects select option labels longer than this.
const MAX_LABEL_CHARS: usize = 100;

/// Lists the results in a select menu and returns the URL the invoker picked.
/// Returns `None` when nobody picks before the timeout.
pub async fn pick_result(
    ctx: Context<'_>,
    results: &[SearchResult],
) -> Result<Option<String>, Error> {
    let serenity_ctx = ctx.serenity_context();

    // The option value is the index into `results`, so the URL never has to
    // fit in Discord's 100 character limit for option values.
    let options = results
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let label: String = result.title.chars().take(MAX_LABEL_CHARS).collect();

            CreateSelectMenuOption::new(label, index.to_string())
        })
        .collect();

    let menu = CreateSelectMenu::new(MENU_ID, CreateSelectMenuKind::String { options })
        .placeholder("Pick a song");

    let message = ctx
        .msg
        .channel_id
        .send_message(
            serenity_ctx,
            CreateMessage::new().content("Search results:").select_menu(menu),
        )
        .await?;

    let interaction = message
        .await_component_interaction(&serenity_ctx.shard)
        .author_id(ctx.author().id)
        .timeout(SELECTION_TIMEOUT)
        .await;

    let Some(interaction) = interaction else {
        message.delete(serenity_ctx).await?;
        return Ok(None);
    };

    // Discord shows "interaction failed" unless the interaction is answered.
    interaction
        .create_response(serenity_ctx, CreateInteractionResponse::Acknowledge)
        .await?;
    message.delete(serenity_ctx).await?;

    let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind else {
        return Err("Unexpected interaction data kind".into());
    };

    let index: usize = values.first().ok_or("Empty song selection")?.parse()?;

    Ok(results.get(index).map(|result| result.url.clone()))
}
