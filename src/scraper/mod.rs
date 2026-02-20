use std::sync::Arc;

use crate::global::Global;
use crate::util::sleep_until_aligned;

pub mod sources;

#[tracing::instrument(name = "Scraper", skip_all)]
pub async fn run(global: Arc<Global>) -> anyhow::Result<()> {
	if !global.config.scraper.enabled {
		tracing::info!("scraper is disabled");
		// Park forever so tokio::select doesn't exit
		std::future::pending::<()>().await;
		return Ok(());
	}

	let interval_secs = global.config.scraper.interval_secs;
	tracing::info!(interval_secs, "starting scraper");

	loop {
		if let Err(e) = sources::genshin::scrape_and_store(&global).await {
			tracing::error!(error = %e, "genshin scraper failed");
		}

		if let Err(e) = sources::starrail::scrape_and_store(&global).await {
			tracing::error!(error = %e, "starrail scraper failed");
		}

		if let Err(e) = sources::zenless::scrape_and_store(&global).await {
			tracing::error!(error = %e, "zenless scraper failed");
		}

		if let Err(e) = sources::themis::scrape_and_store(&global).await {
			tracing::error!(error = %e, "themis scraper failed");
		}

		sleep_until_aligned(interval_secs).await;
	}
}
