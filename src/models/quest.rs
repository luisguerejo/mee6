use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Quest {
    pub id: String,
    pub name: String,
}
