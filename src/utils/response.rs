use crate::models::task::Task;

pub fn format_task_response(task: &Task) -> String {
    let keys_output = if task.neededKeys.is_empty() {
        "None".to_string()
    } else {
        task.neededKeys
            .iter()
            .flat_map(|wrapper| &wrapper.keys)
            .map(|key| {
                format!(
                    "\n- **{}**\n  - avg price: {}\n    - **[Wiki Link]({})**",
                    key.name,
                    key.avg24hPrice.unwrap_or(0),
                    key.wikiLink.as_deref().unwrap_or("#")
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    format!(
        "**Quest:** {}\n\
        **Kappa Required:** `{}`\n\
        **Needed Keys**: {}\n\
        **[Wiki Link]({})**",
        task.name, task.kappaRequired, keys_output, task.wikiLink
    )
}
