use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

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

impl From<GameCode> for GameCodeResponse {
    fn from(code: GameCode) -> Self {
        Self {
            code: code.code,
            rewards: code.rewards,
        }
    }
}
