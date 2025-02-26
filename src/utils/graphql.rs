use crate::models::task::TaskResponse;
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

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
