use crate::{types::{GameCode, NewsItem}, config::Settings};

pub mod codes;
pub mod news;

pub struct GenshinResolver;

impl GenshinResolver {
    pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
        codes::fetch_codes(config).await
    }

    pub async fn fetch_news(config: &Settings, category: &str) -> anyhow::Result<Vec<NewsItem>> {
        news::fetch_news(config, category).await
    }
} 