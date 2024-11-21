use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarResponse {
    pub events: Vec<Event>,
    pub banners: Vec<Banner>,
    pub challenges: Vec<Challenge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub type_name: String,
    pub start_time: i64,
    pub end_time: i64,
    pub rewards: Vec<Reward>,
    pub special_reward: Option<Reward>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Banner {
    pub id: String,
    pub name: String,
    pub version: String,
    pub characters: Vec<Character>,
    pub start_time: i64,
    pub end_time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub element: String,
    pub icon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Challenge {
    pub id: i32,
    pub name: String,
    pub type_name: String,
    pub start_time: i64,
    pub end_time: i64,
    pub rewards: Vec<Reward>,
    pub special_reward: Option<Reward>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reward {
    pub id: i32,
    pub name: String,
    pub icon: String,
    pub rarity: String,
    pub amount: i32,
}