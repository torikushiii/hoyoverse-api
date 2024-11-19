use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GenshinCalendarResponse {
    pub retcode: i32,
    pub message: String,
    pub data: Option<GenshinCalendarData>,
}

#[derive(Debug, Deserialize)]
pub struct GenshinCalendarData {
    pub avatar_card_pool_list: Vec<BannerPool>,
    pub weapon_card_pool_list: Vec<BannerPool>,
    pub mixed_card_pool_list: Vec<BannerPool>,
    pub selected_avatar_card_pool_list: Vec<BannerPool>,
    pub selected_mixed_card_pool_list: Vec<BannerPool>,
    pub act_list: Vec<GameEvent>,
    pub fixed_act_list: Vec<GameEvent>,
    pub selected_act_list: Vec<GameEvent>,
}

#[derive(Debug, Deserialize)]
pub struct BannerPool {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub pool_type: String,
    pub version: String,
    pub start_timestamp: String,
    pub end_timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct GameEvent {
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub desc: String,
    pub start_timestamp: String,
    pub end_timestamp: String,
    pub reward_list: Vec<GameReward>,
}

#[derive(Debug, Deserialize)]
pub struct GameReward {
    pub item_id: i32,
    pub name: String,
    pub icon: String,
    pub wiki_url: String,
    pub num: i32,
    pub rarity: String,
}