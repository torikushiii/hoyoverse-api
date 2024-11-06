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
    #[serde(rename = "created_at")]
    pub created_at: i64,
    pub banner: Vec<String>,
    pub url: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub lang: String,
}

impl From<GameCode> for GameCodeResponse {
    fn from(code: GameCode) -> Self {
        Self {
            code: code.code,
            rewards: code.rewards,
        }
    }
} 