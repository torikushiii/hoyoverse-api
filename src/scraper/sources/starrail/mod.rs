use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use futures::TryStreamExt as _;
use mongodb::bson::doc;

use crate::database::redemption_code::RedemptionCode;
use crate::games::Game;
use crate::global::Global;
use crate::validator::hoyoverse_api;

pub mod fandom;
pub mod game8;

#[tracing::instrument(name = "starrail", skip_all)]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<()> {
    let (fandom_result, game8_result) = tokio::join!(fandom::scrape(global), game8::scrape(global));

    let mut all_codes: HashMap<String, (Vec<String>, &'static str)> = HashMap::new();

    match fandom_result {
        Ok(scraped) => {
            for p in scraped {
                all_codes
                    .entry(p.code.to_uppercase())
                    .or_insert((p.rewards, "fandom"));
            }
        }
        Err(e) => tracing::error!(error = %e, "fandom scraper failed"),
    }

    match game8_result {
        Ok(scraped) => {
            for p in scraped {
                all_codes
                    .entry(p.code.to_uppercase())
                    .or_insert((p.rewards, "game8"));
            }
        }
        Err(e) => tracing::error!(error = %e, "game8 scraper failed"),
    }

    if all_codes.is_empty() {
        return Ok(());
    }

    let collection = RedemptionCode::collection(&global.db, Game::Starrail);
    let total = all_codes.len();

    let candidates: Vec<String> = all_codes.keys().cloned().collect();
    let existing: HashSet<String> = collection
        .find(doc! { "code": { "$in": &candidates } })
        .await?
        .try_collect::<Vec<RedemptionCode>>()
        .await?
        .into_iter()
        .map(|c| c.code)
        .collect();

    let new_codes: Vec<(String, Vec<String>, &'static str)> = all_codes
        .into_iter()
        .filter(|(code, _)| !existing.contains(code))
        .map(|(code, (rewards, source))| (code, rewards, source))
        .collect();

    if new_codes.is_empty() {
        tracing::info!(total, "starrail scrape complete, no new codes");
        return Ok(());
    }

    let validation_enabled = global
        .config
        .validator
        .game_config(Game::Starrail)
        .is_some_and(|c| c.enabled)
        && Game::Starrail.redeem_endpoint().is_some();

    let mut new_count = 0;

    for (code, rewards, source) in &new_codes {
        if validation_enabled {
            let valid = loop {
                match hoyoverse_api::validate_code(global, Game::Starrail, code).await {
                    Ok(resp) if resp.is_cooldown() => {
                        tracing::warn!(code, "hit cooldown, retrying in 6s");
                        tokio::time::sleep(std::time::Duration::from_secs(6)).await;
                        continue;
                    }
                    Ok(resp) => break resp.is_code_valid(),
                    Err(e) => {
                        tracing::warn!(code, error = %e, "validation request failed, inserting anyway");
                        break true;
                    }
                }
            };

            if !valid {
                tracing::warn!(code, "code is invalid, storing as inactive");
                collection
                    .insert_one(RedemptionCode {
                        code: code.clone(),
                        active: false,
                        date: bson::DateTime::now(),
                        rewards: rewards.clone(),
                        source: source.to_string(),
                    })
                    .await?;
                tokio::time::sleep(std::time::Duration::from_secs(6)).await;
                continue;
            }

            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }

        let doc = RedemptionCode {
            code: code.clone(),
            active: true,
            date: bson::DateTime::now(),
            rewards: rewards.clone(),
            source: source.to_string(),
        };

        collection.insert_one(doc).await?;
        tracing::info!(code, source, "new code discovered");
        new_count += 1;
    }

    tracing::info!(new = new_count, total, "starrail scrape complete");

    if new_count > 0 {
        global
            .response_cache
            .remove(&format!("/mihoyo/{}/codes", Game::Starrail.slug()))
            .await;
    }

    Ok(())
}
