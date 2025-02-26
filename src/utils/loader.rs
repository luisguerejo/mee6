use crate::models::ammo::Ammo;
use crate::models::quest::Quest;
use serde_json::Error;
use std::fs;

pub async fn load_quests() -> Result<Vec<Quest>, Error> {
    let file_content = fs::read_to_string("../quests.json")
        .map_err(|e| {
            eprintln!("Failed to read JSON file: {}", e);
            e
        })
        .expect("Error loading JSON file: quests.json");

    let quests: Vec<Quest> = serde_json::from_str(&file_content)?;
    Ok(quests)
}

pub async fn load_ammo() -> Result<Vec<Ammo>, Error> {
    todo!()
}
