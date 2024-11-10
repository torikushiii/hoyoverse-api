use serde::{Deserialize, Serialize};
use mongodb::bson::{oid::ObjectId, DateTime};

// Internal model for database
#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewsItem {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub external_id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    pub banner: Option<Vec<String>>,
    pub url: String,
    #[serde(rename = "type")]
    pub news_type: String,
    pub lang: String,
}

#[derive(Debug, Deserialize)]
pub struct HoyolabResponse<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct EventList {
    pub list: Vec<EventItem>,
}

#[derive(Debug, Deserialize)]
pub struct EventItem {
    pub id: String,
    pub name: String,
    pub desc: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub create_at: i64,
    pub banner_url: String,
}

#[derive(Debug, Deserialize)]
pub struct NewsPost {
    pub post: Post,
    pub image_list: Vec<ImageItem>,
}

#[derive(Debug, Deserialize)]
pub struct Post {
    pub post_id: String,
    pub subject: String,
    pub content: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ImageItem {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct NewsList {
    pub list: Vec<NewsPost>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewsItemResponse {
    pub id: String,  // This will be the external_id
    pub title: String,
    pub description: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    pub banner: Option<Vec<String>>,
    pub url: String,
    #[serde(rename = "type")]
    pub news_type: String,
}

impl From<GameCode> for GameCodeResponse {
    fn from(code: GameCode) -> Self {
        Self {
            code: code.code,
            rewards: code.rewards,
        }
    }
}

impl From<NewsItem> for NewsItemResponse {
    fn from(item: NewsItem) -> Self {
        Self {
            id: item.external_id,
            title: item.title,
            description: item.description,
            created_at: item.created_at,
            banner: item.banner,
            url: item.url,
            news_type: item.news_type,
        }
    }
}

fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TimestampFormat {
        String(String),
        Integer(i64),
    }

    match TimestampFormat::deserialize(deserializer)? {
        TimestampFormat::String(s) => s.parse::<i64>()
            .map_err(|e| Error::custom(format!("Failed to parse string timestamp: {}", e))),
        TimestampFormat::Integer(i) => Ok(i),
    }
}