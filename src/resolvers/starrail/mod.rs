pub mod codes;
pub mod news;
pub mod calendar;

use crate::{
    types::{GameCode, NewsItem, CalendarResponse},
    config::Settings,
};

pub struct StarRailResolver;

impl StarRailResolver {
    pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
        codes::fetch_codes(config).await
    }

    pub async fn fetch_news(config: &Settings, category: &str) -> anyhow::Result<Vec<NewsItem>> {
        news::fetch_news(config, category).await
    }

    pub async fn fetch_calendar(config: &Settings) -> anyhow::Result<CalendarResponse> {
        calendar::fetch_calendar(config).await
    }
}