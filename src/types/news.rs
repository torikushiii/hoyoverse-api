use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

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
pub struct NewsPost {
    pub post: Post,
    pub image_list: Vec<ImageItem>,
}

#[derive(Debug, Deserialize)]
pub struct Post {
    pub post_id: String,
    pub subject: String,
    pub content: String,
    #[serde(deserialize_with = "crate::types::hoyolab::deserialize_timestamp")]
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
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    pub banner: Option<Vec<String>>,
    pub url: String,
    #[serde(rename = "type")]
    pub news_type: String,
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
