pub mod sources;

use crate::types::NewsItem;
use sources::hoyolab;

pub async fn fetch_news(category: &str) -> anyhow::Result<Vec<NewsItem>> {
    hoyolab::fetch_news(category).await
} 