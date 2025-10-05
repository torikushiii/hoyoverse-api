pub mod calendar;
pub mod codes;
pub mod news;

use crate::{
    config::Settings,
    db::MongoConnection,
    types::{calendar::StarRailCalendarResponse, GameCode, NewsItem},
};

pub struct StarRailResolver;

impl StarRailResolver {
    pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
        codes::fetch_codes(config).await
    }

    pub async fn fetch_news(config: &Settings, category: &str) -> anyhow::Result<Vec<NewsItem>> {
        news::fetch_news(config, category).await
    }

    pub async fn fetch_calendar(
        config: &Settings,
        mongo: &MongoConnection,
    ) -> anyhow::Result<StarRailCalendarResponse> {
        calendar::fetch_calendar(config, mongo).await
    }
}
