use std::sync::Arc;

use crate::global::Global;

pub mod fandom;
pub mod game8;

#[tracing::instrument(name = "genshin", skip_all)]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<()> {
    if let Err(e) = fandom::scrape_and_store(global).await {
        tracing::error!(error = %e, "fandom scraper failed");
    }

    if let Err(e) = game8::scrape_and_store(global).await {
        tracing::error!(error = %e, "game8 scraper failed");
    }

    Ok(())
}
