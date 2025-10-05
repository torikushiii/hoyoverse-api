use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://game8.co/games/Genshin-Impact/archives/304759";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Game8 page")?;

    if !response.status().is_success() {
        warn!(
            "[Genshin][Codes][Game8] Failed to fetch data, status: {}",
            response.status()
        );
        return Ok(Vec::new());
    }

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let mut codes = Vec::new();
    let current_time = Utc::now();

    let list_selector = Selector::parse("ol.a-orderedList").unwrap();
    let item_selector = Selector::parse("li.a-listItem").unwrap();
    let link_selector = Selector::parse("a.a-link").unwrap();
    let expired_selector = Selector::parse("span.a-red").unwrap();

    for list in document.select(&list_selector) {
        for item in list.select(&item_selector) {
            if item.select(&expired_selector).next().is_some() {
                continue;
            }

            let code = if let Some(link) = item.select(&link_selector).next() {
                link.text().collect::<String>()
            } else {
                continue;
            };

            let item_text = item.text().collect::<String>();

            if let Some(code_pos) = item_text.find(&code) {
                if let Some(dash_pos) = item_text[code_pos..].find('-') {
                    let rewards_text = &item_text[code_pos + dash_pos + 1..];
                    let mut rewards: Vec<String> = Vec::new();

                    let normalized_text = rewards_text.replace(" and ", ", ");
                    let parts: Vec<&str> = normalized_text.split(',').collect();

                    let mut current_reward = String::new();
                    for part in parts {
                        let part = part.trim();
                        if part.is_empty() || part.contains("EXPIRED") {
                            continue;
                        }

                        if part.chars().next().map_or(false, |c| c.is_ascii_digit())
                            && !current_reward.is_empty()
                            && current_reward
                                .chars()
                                .last()
                                .map_or(false, |c| c.is_ascii_digit())
                        {
                            current_reward.push(',');
                            current_reward.push_str(part);
                        } else {
                            // If we have a previous reward, push it
                            if !current_reward.is_empty() {
                                rewards.push(current_reward);
                            }
                            current_reward = part.to_string();
                        }
                    }

                    if !current_reward.is_empty() {
                        rewards.push(current_reward);
                    }

                    if !code.is_empty() && !rewards.is_empty() {
                        codes.push(GameCode {
                            id: None,
                            code,
                            active: true,
                            date: current_time.into(),
                            rewards,
                            source: "game8".to_string(),
                        });
                    }
                }
            }
        }
    }

    if codes.is_empty() {
        warn!("[Genshin][Codes][Game8] No codes found");
    } else {
        debug!(
            "[Genshin][Codes][Game8] Fetched {} codes total",
            codes.len()
        );
    }

    Ok(codes)
}
