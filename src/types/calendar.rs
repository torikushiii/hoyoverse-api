use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarResponse {
    pub events: Vec<Event>,
    #[serde(rename = "banners")]
    pub genshin_banners: Vec<GenshinBanner>,
    pub challenges: Vec<Challenge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StarRailCalendarResponse {
    pub events: Vec<Event>,
    pub banners: Vec<StarRailBanner>,
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
pub struct GenshinBanner {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub characters: Vec<Character>,
    #[serde(default)]
    pub weapons: Vec<GenshinWeapon>,
    pub start_time: i64,
    pub end_time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StarRailBanner {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub characters: Vec<Character>,
    #[serde(default)]
    pub light_cones: Vec<LightCone>,
    pub start_time: i64,
    pub end_time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub element: String,
    pub path: Option<String>,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenshinWeapon {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightCone {
    pub id: String,
    pub name: String,
    pub rarity: String,
    pub path: String,
    pub icon: String,
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