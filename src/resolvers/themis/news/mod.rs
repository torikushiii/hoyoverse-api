pub mod sources;

use crate::{types::NewsItem, config::Settings};

pub async fn fetch_news(config: &Settings, category: &str) -> anyhow::Result<Vec<NewsItem>> {
    sources::hoyolab::fetch_news(config, category).await
}