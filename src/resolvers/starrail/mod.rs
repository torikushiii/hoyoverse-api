pub mod codes;
pub mod news;

use crate::types::{GameCode, NewsItem};

pub struct StarRailResolver;

impl StarRailResolver {
    pub async fn fetch_codes() -> anyhow::Result<Vec<GameCode>> {
        codes::fetch_all_codes().await
    }

    pub async fn fetch_news(category: &str) -> anyhow::Result<Vec<NewsItem>> {
        news::fetch_news(category).await
    }
} 