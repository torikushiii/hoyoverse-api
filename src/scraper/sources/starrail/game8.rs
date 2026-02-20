use std::sync::Arc;

use mongodb::bson::doc;
use regex::Regex;

use crate::database::redemption_code::RedemptionCode;
use crate::games::Game;
use crate::global::Global;
use crate::validator::hoyoverse_api;

const GAME8_URL: &str = "https://game8.co/games/Honkai-Star-Rail/archives/410296";
const EXPIRED_MARKER: &str = "All Expired Star Rail Redeem Codes";
const SOURCE: &str = "game8";

#[derive(Debug)]
pub struct ParsedCode {
    pub code: String,
    pub rewards: Vec<String>,
}

#[tracing::instrument(skip(global))]
pub async fn scrape(global: &Arc<Global>) -> anyhow::Result<Vec<ParsedCode>> {
    let html = global
        .http_client
        .get(GAME8_URL)
        .send()
        .await?
        .text()
        .await?;

    let codes = parse_html(&html);

    tracing::info!(count = codes.len(), "scraped codes from game8");

    Ok(codes)
}

pub fn parse_html(html: &str) -> Vec<ParsedCode> {
    let active_html = match html.find(EXPIRED_MARKER) {
        Some(pos) => &html[..pos],
        None => html,
    };

    let row_re =
        Regex::new(r"(?s)<tr>\s*<td.*?>(.*?)</td>\s*<td.*?>(.*?)</td>").expect("invalid row regex");
    let code_re = Regex::new(r#"value=['"]([^'"]+)['"]"#).expect("invalid code regex");
    let fallback_code_re =
        Regex::new(r"gift\?code=([A-Z0-9]{4,})").expect("invalid fallback code regex");
    let reward_re =
        Regex::new(r#"(?s)<div class=['"]align['"]>.*?<a.*?>(.*?)</a>\s*x?\s*([\d,]+)"#)
            .expect("invalid reward regex");
    let tag_re = Regex::new(r"<[^>]*>").expect("invalid tag regex");

    let mut results = Vec::new();

    for cap in row_re.captures_iter(active_html) {
        let code_td = &cap[1];
        let rewards_td = &cap[2];

        let code = code_re
            .captures(code_td)
            .map(|c| c[1].to_string())
            .or_else(|| fallback_code_re.captures(code_td).map(|c| c[1].to_string()));

        if let Some(code) = code {
            let mut rewards = Vec::new();
            for r_cap in reward_re.captures_iter(rewards_td) {
                let name_html = &r_cap[1];
                let qty = &r_cap[2];
                let name = tag_re.replace_all(name_html, "").trim().to_string();

                if !name.is_empty() {
                    let qty = qty.trim_start_matches('x');
                    rewards.push(format!("{} ×{}", name, qty));
                }
            }

            results.push(ParsedCode {
                code: code.to_uppercase(),
                rewards,
            });
        }
    }

    results.sort_by(|a, b| a.code.cmp(&b.code));
    results.dedup_by(|a, b| a.code == b.code);

    results
}

#[tracing::instrument(skip(global))]
pub async fn scrape_and_store(global: &Arc<Global>) -> anyhow::Result<usize> {
    let scraped = scrape(global).await?;
    let collection = RedemptionCode::collection(&global.db, Game::Starrail);

    let mut new_codes: Vec<(String, Vec<String>)> = Vec::new();

    for parsed in &scraped {
        let exists = collection
            .count_documents(doc! { "code": &parsed.code })
            .await?
            > 0;

        if !exists {
            new_codes.push((parsed.code.clone(), parsed.rewards.clone()));
        }
    }

    if new_codes.is_empty() {
        tracing::info!(total = scraped.len(), "game8 scrape complete, no new codes");
        return Ok(0);
    }

    let validation_enabled = global
        .config
        .validator
        .game_config(Game::Starrail)
        .is_some_and(|c| c.enabled)
        && Game::Starrail.redeem_endpoint().is_some();

    let mut new_count = 0;

    for (code, rewards) in &new_codes {
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
                tracing::info!(code, "skipping invalid code");
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
            source: SOURCE.to_string(),
        };

        collection.insert_one(doc).await?;
        tracing::info!(code, "new code discovered");
        new_count += 1;
    }

    tracing::info!(
        new = new_count,
        total = scraped.len(),
        "game8 scrape complete"
    );

    Ok(new_count)
}
