use std::collections::HashSet;
use std::sync::Arc;

use futures::TryStreamExt as _;
use mongodb::bson::doc;

use crate::database::redemption_code::RedemptionCode;
use crate::games::Game;
use crate::global::Global;
use crate::notifier::discord;

pub mod tot_wiki;

#[tracing::instrument(name = "themis", skip_all)]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<()> {
    let scraped = match tot_wiki::scrape(global).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "tot_wiki scraper failed");
            return Ok(());
        }
    };

    if scraped.is_empty() {
        return Ok(());
    }

    let collection = RedemptionCode::collection(&global.db, Game::Themis);
    let total = scraped.len();

    let candidates: Vec<String> = scraped.iter().map(|p| p.code.clone()).collect();
    let existing: HashSet<String> = collection
        .find(doc! { "code": { "$in": &candidates } })
        .await?
        .try_collect::<Vec<RedemptionCode>>()
        .await?
        .into_iter()
        .map(|c| c.code)
        .collect();

    let mut new_count = 0;
    let mut new_valid_codes: Vec<(String, Vec<String>, String)> = Vec::new();

    for parsed in scraped.iter().filter(|p| !existing.contains(&p.code)) {
        let doc = RedemptionCode {
            code: parsed.code.clone(),
            active: true,
            date: bson::DateTime::now(),
            rewards: parsed.rewards.clone(),
            source: "tot_wiki".to_string(),
        };

        collection.insert_one(doc).await?;
        tracing::info!(code = parsed.code, "new code discovered");
        metrics::counter!("scraper_codes_discovered_total", "game" => Game::Themis.slug()).increment(1);
        new_valid_codes.push((parsed.code.clone(), parsed.rewards.clone(), "tot_wiki".to_string()));
        new_count += 1;
    }

    tracing::info!(new = new_count, total, "themis scrape complete");

    if new_count > 0 {
        discord::notify_new_codes(global, Game::Themis, &new_valid_codes).await;
        global
            .response_cache
            .remove(&format!("/mihoyo/{}/codes", Game::Themis.slug()))
            .await;
    }

    Ok(())
}
