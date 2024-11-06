use serde::{Serialize, Deserialize};
use mongodb::bson::{DateTime, oid::ObjectId};

// Internal model for database
#[derive(Debug, Serialize, Deserialize)]
pub struct GameCode {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub code: String,
    pub active: bool,
    pub date: DateTime,
    pub rewards: Vec<String>,
    pub source: String,
}

// API response model
#[derive(Debug, Serialize, Deserialize)]
pub struct GameCodeResponse {
    pub code: String,
    pub rewards: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodesResponse {
    pub active: Vec<GameCodeResponse>,
    pub inactive: Vec<GameCodeResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "startAt", skip_serializing_if = "Option::is_none")]
    pub start_at: Option<i64>,
    #[serde(rename = "endAt", skip_serializing_if = "Option::is_none")]
    pub end_at: Option<i64>,
    pub banner: Vec<String>,
    pub url: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub lang: String,
}

#[derive(Debug, Deserialize)]
pub struct HoyolabResponse<T> {
    pub retcode: i32,
    pub message: String,
    pub data: HoyolabData<T>,
}

#[derive(Debug, Deserialize)]
pub struct HoyolabData<T> {
    pub list: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct EventItem {
    pub id: String,
    pub name: String,
    pub desc: String,
    #[serde(deserialize_with = "deserialize_string_to_i64")]
    pub create_at: i64,
    #[serde(deserialize_with = "deserialize_string_to_i64")]
    pub start: i64,
    #[serde(deserialize_with = "deserialize_string_to_i64")]
    pub end: i64,
    pub banner_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PostItem {
    pub post_id: String,
    pub subject: String,
    pub content: String,
    pub created_at: i64,
    #[serde(default)]
    pub image_list: Vec<ImageItem>,
}

#[derive(Debug, Deserialize)]
pub struct NewsListItem {
    pub post: PostItem,
    #[serde(default)]
    pub image_list: Vec<ImageItem>,
}

#[derive(Debug, Deserialize)]
pub struct ImageItem {
    pub url: String,
}

impl From<GameCode> for GameCodeResponse {
    fn from(code: GameCode) -> Self {
        Self {
            code: code.code,
            rewards: code.rewards,
        }
    }
}

fn deserialize_string_to_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    String::deserialize(deserializer)?
        .parse::<i64>()
        .map_err(Error::custom)
} 