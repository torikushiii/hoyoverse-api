use std::sync::Arc;

use crate::games::Game;
use crate::global::Global;
use crate::scraper::store::{StoreOptions, store_codes};

pub mod tot_wiki;

#[tracing::instrument(name = "themis", skip_all)]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<()> {
    let result = tot_wiki::scrape(global).await;

    store_codes(
        global,
        Game::Themis,
        vec![("tot_wiki", result)],
        StoreOptions {
            uppercase: false,
            validate: false,
            sync_active: false,
        },
    )
    .await
}
