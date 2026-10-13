use std::sync::Arc;

use crate::games::Game;
use crate::global::Global;
use crate::scraper::store::{StoreOptions, store_codes};

use super::crimson_witch;

const CRIMSON_WITCH_URL: &str = "https://www.crimsonwitch.com/codes/Honkai_Star_Rail";

pub mod fandom;
pub mod game8;
pub mod hoyolab;
pub mod sportskeeda;

#[tracing::instrument(name = "starrail", skip_all)]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<()> {
    let (fandom_result, game8_result, sportskeeda_result, hoyolab_result, crimson_witch_result) = tokio::join!(
        fandom::scrape(global),
        game8::scrape(global),
        sportskeeda::scrape(global),
        hoyolab::scrape(global),
        crimson_witch::scrape(&global.http_client, CRIMSON_WITCH_URL),
    );

    store_codes(
        global,
        Game::Starrail,
        vec![
            ("fandom", fandom_result),
            ("game8", game8_result),
            ("sportskeeda", sportskeeda_result),
            ("hoyolab", hoyolab_result),
            ("crimson_witch", crimson_witch_result),
        ],
        StoreOptions {
            uppercase: true,
            validate: true,
            sync_active: false,
        },
    )
    .await
}
