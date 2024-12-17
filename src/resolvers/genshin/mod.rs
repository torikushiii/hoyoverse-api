use crate::{
    types::{GameCode, NewsItem, CalendarResponse},
    config::Settings,
    db::MongoConnection,
};

pub mod codes;
pub mod news;
pub mod calendar;

pub struct GenshinResolver;

impl GenshinResolver {
    pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
        codes::fetch_codes(config).await
    }

    pub async fn fetch_news(config: &Settings, category: &str) -> anyhow::Result<Vec<NewsItem>> {
        news::fetch_news(config, category).await
    }

    pub async fn fetch_calendar(config: &Settings, mongo: &MongoConnection) -> anyhow::Result<CalendarResponse> {
        calendar::fetch_calendar(config, mongo).await
    }
}