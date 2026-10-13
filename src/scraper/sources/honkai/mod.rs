use std::sync::Arc;

use crate::games::Game;
use crate::global::Global;
use crate::scraper::store::{StoreOptions, store_codes};

pub mod fandom;

#[tracing::instrument(name = "honkai", skip_all)]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<()> {
    let result = fandom::scrape(global).await;

    store_codes(
        global,
        Game::Honkai,
        vec![("fandom", result)],
        StoreOptions {
            uppercase: false,
            validate: false,
            sync_active: true,
        },
    )
    .await
}
