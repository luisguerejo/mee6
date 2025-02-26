use crate::tarkov::types::{Quest, Task, TaskResponse};
use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use serde_json::Error;
use std::fs;

pub async fn load_quests() -> Result<Vec<Quest>, Error> {
    let file_content = fs::read_to_string("data/quests.json")
        .map_err(|e| {
            eprintln!("Failed to read JSON file: {}", e);
            e
        })
        .expect("Error loading JSON file: quests.json");

    let quests: Vec<Quest> = serde_json::from_str(&file_content)?;
    Ok(quests)
}

pub async fn fetch_task(id: &str) -> Result<TaskResponse> {
    let client = Client::new();
    let query_string = format!(
        r#"{{
            task(id: "{}") {{
                name
                minPlayerLevel
                kappaRequired
                wikiLink
                neededKeys {{
                    keys {{
                        name
                        avg24hPrice
                        wikiLink
                    }}
                }}
            }}
        }}"#,
        id
    );

    let query = json!({ "query": query_string });
    let res = client
        .post("https://api.tarkov.dev/graphql")
        .header("Content-Type", "application/json")
        .json(&query)
        .send()
        .await?;

    let task_response: TaskResponse = res.json().await?;

    Ok(task_response)
}

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
