use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CodeStatus {
    #[default]
    Active,
    Inactive,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameCode {
    pub code: String,
    #[serde(default)]
    pub rewards: Vec<String>,
    pub source: String,
    pub date: DateTime<Utc>,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodesResponse {
    pub active: Vec<GameCode>,
    pub inactive: Vec<GameCode>,
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