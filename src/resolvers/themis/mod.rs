pub mod codes;
pub mod news;

use crate::{
    types::{GameCode, NewsItem},
    config::Settings,
};

pub struct ThemisResolver;

impl ThemisResolver {
    pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
        codes::fetch_codes(config).await
    }

    pub async fn fetch_news(config: &Settings, category: &str) -> anyhow::Result<Vec<NewsItem>> {
        news::fetch_news(config, category).await
    }
}