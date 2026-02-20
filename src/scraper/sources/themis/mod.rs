use std::sync::Arc;

use crate::global::Global;

pub mod tot_wiki;

/// Scrape and store codes from all Tears of Themis sources.
#[tracing::instrument(name = "themis", skip_all)]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<()> {
    if let Err(e) = tot_wiki::scrape_and_store(global).await {
        tracing::error!(error = %e, "tot_wiki scraper failed");
    }

    Ok(())
}
