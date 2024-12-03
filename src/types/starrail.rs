use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StarRailCalendarResponse {
    pub retcode: i32,
    pub message: String,
    pub data: Option<StarRailCalendarData>,
}

#[derive(Debug, Deserialize)]
pub struct StarRailCalendarData {
    pub avatar_card_pool_list: Vec<CharacterBannerPool>,
    pub equip_card_pool_list: Vec<LightConeBannerPool>,
    pub act_list: Vec<GameEvent>,
    pub challenge_list: Vec<GameChallenge>,
    pub now: String,
    pub cur_game_version: String,
}

#[derive(Debug, Deserialize)]
pub struct CharacterBannerPool {
    pub name: String,
    #[serde(rename = "type")]
    pub pool_type: String,
    pub avatar_list: Vec<Character>,
    #[serde(default)]
    pub equip_list: Vec<LightCone>,
    pub is_after_version: bool,
    pub time_info: TimeInfo,
    pub version: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct LightConeBannerPool {
    pub name: String,
    #[serde(rename = "type")]
    pub pool_type: String,
    #[serde(default)]
    pub avatar_list: Vec<Character>,
    pub equip_list: Vec<LightCone>,
    pub is_after_version: bool,
    pub time_info: TimeInfo,
    pub version: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct Character {
    pub item_id: String,
    pub item_name: String,
    pub icon_url: String,
    pub damage_type: String,
    pub rarity: String,
    pub avatar_base_type: String,
    pub is_forward: bool,
    pub wiki_url: String,
    pub item_avatar_icon_path: String,
    pub damage_type_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LightCone {
    pub item_id: String,
    pub item_name: String,
    #[serde(rename = "item_url")]
    pub icon_url: String,
    pub avatar_base_type: String,
    pub rarity: String,
    pub is_forward: bool,
    pub wiki_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GameEvent {
    pub id: i32,
    pub version: String,
    pub name: String,
    pub act_type: String,
    pub act_status: String,
    pub reward_list: Vec<GameReward>,
    pub total_progress: i32,
    pub current_progress: i32,
    pub time_info: TimeInfo,
    pub panel_id: i32,
    pub panel_desc: String,
    pub is_after_version: bool,
    pub sort_weight: i32,
    pub special_reward: GameReward,
}

#[derive(Debug, Deserialize)]
pub struct GameChallenge {
    pub group_id: i32,
    pub name_mi18n: String,
    pub challenge_type: String,
    pub total_progress: i32,
    pub current_progress: i32,
    pub status: String,
    pub time_info: TimeInfo,
    pub reward_list: Vec<GameReward>,
    pub special_reward: GameReward,
}

#[derive(Debug, Deserialize)]
pub struct TimeInfo {
    pub start_ts: String,
    pub end_ts: String,
    pub start_time: String,
    pub end_time: String,
    pub now: String,
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