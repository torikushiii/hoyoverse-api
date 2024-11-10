use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://game8.co/games/Honkai-Star-Rail/archives/410296";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Game8 page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let title_selector = Selector::parse("h2").unwrap();
    let list_selector = Selector::parse("ul.a-list").unwrap();
    let item_selector = Selector::parse("li.a-listItem").unwrap();
    let link_selector = Selector::parse("a.a-link").unwrap();

    let mut codes = Vec::new();
    let current_time = Utc::now();

    // Find the correct section by title
    for title in document.select(&title_selector) {
        if title.text().collect::<String>().contains("Active Redeem Codes for") {
            if let Some(code_list) = document.select(&list_selector).next() {
                for item in code_list.select(&item_selector) {
                    // Get the code from the first a.a-link element
                    if let Some(code_element) = item.select(&link_selector).next() {
                        let code = code_element.text().collect::<String>().trim().to_string();

                        // Get rewards text by removing the code and "NEW" from the full text
                        let full_text = item.text().collect::<String>();
                        let rewards_text = full_text
                            .replace(&code, "")
                            .replace("NEW", "")
                            .trim()
                            .to_string();

                        // Clean up rewards text
                        let rewards_text = rewards_text
                            .trim_start_matches('(')
                            .trim_end_matches(')')
                            .to_string();

                        // Process rewards, handling cases where numbers might be split by commas
                        let mut rewards = Vec::new();
                        let mut current_reward = String::new();

                        for part in rewards_text.split(',') {
                            let part = part.trim();
                            if current_reward.chars().last().map_or(false, |c| c.is_ascii_digit())
                               && part.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                                // If current reward ends with number and next part starts with number,
                                // treat it as a thousands separator
                                current_reward.push(',');
                                current_reward.push_str(part);
                            } else {
                                if !current_reward.is_empty() {
                                    rewards.push(current_reward.trim().to_string());
                                }
                                current_reward = part.to_string();
                            }
                        }
                        if !current_reward.is_empty() {
                            rewards.push(current_reward.trim().to_string());
                        }

                        codes.push(GameCode {
                            id: None,
                            code: code.to_string(),
                            active: true,
                            date: current_time.into(),
                            rewards: rewards.clone(),
                            source: "game8".to_string(),
                        });
                    }
                }
            }
            break; // We found and processed the section we wanted
        }
    }

    if codes.is_empty() {
        warn!("[StarRail][Codes][Game8] No codes found");
    } else {
        debug!("[StarRail][Codes][Game8] Found {} codes", codes.len());
    }

    Ok(codes)
}