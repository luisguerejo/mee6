use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TaskResponse {
    pub data: TaskData,
}

#[derive(Debug, Deserialize)]
pub struct TaskData {
    pub task: Task,
}

#[derive(Debug, Deserialize)]
pub struct Task {
    pub name: String,
    pub kappaRequired: bool,
    pub wikiLink: String,
    pub neededKeys: Vec<NeededKeysWrapper>,
}

#[derive(Debug, Deserialize)]
pub struct NeededKeysWrapper {
    pub keys: Vec<Key>,
}

#[derive(Debug, Deserialize)]
pub struct Key {
    pub name: String,
    pub avg24hPrice: Option<i64>,
    pub wikiLink: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Quest {
    pub id: String,
    pub name: String,
}

pub struct Ammo {
    pub caliber: String,
    pub damage: u64,
    pub armor_damage: u64,
    pub penetration_power: u64,
    pub recoil: u64,
}
