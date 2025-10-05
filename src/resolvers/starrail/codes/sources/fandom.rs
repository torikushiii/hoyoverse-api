use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://honkai-star-rail.fandom.com/wiki/Redemption_Code";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch Fandom page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    let current_time = Utc::now();

    let mut codes = Vec::new();

    // Create regex patterns
    let code_regex = Regex::new(r"HSRGRANDOPEN[0-9]|[A-Z0-9]{11,15}").unwrap();
    let amount_regex = Regex::new(r"×\d+").unwrap();
    let citation_regex = Regex::new(r"\[\d+\]").unwrap();

    // Select the table containing the codes
    let table_selector =
        Selector::parse("#mw-content-text > div.mw-parser-output > table > tbody > tr").unwrap();

    for row in document.select(&table_selector) {
        let text = row.text().collect::<String>();

        // Skip China-specific codes
        if text.contains("China") {
            continue;
        }

        // Clean the text by removing unwanted patterns
        let clean_text = text
            .replace("All", "")
            .replace("Quick Redeem", "")
            .replace("Code", "")
            .replace("Server", "")
            .replace("Rewards", "")
            .replace("Duration", "");

        // Remove citation references
        let clean_text = citation_regex.replace_all(&clean_text, "").to_string();
        let clean_text = clean_text.trim().to_string();

        if let Some(code_match) = code_regex.find(&clean_text) {
            let code = code_match.as_str().to_string();

            // Extract rewards text and split by "Discovered"
            let rewards_text = clean_text[code_match.end()..]
                .trim()
                .split("Discovered")
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            // Split rewards by the amount pattern
            let reward_parts: Vec<&str> = amount_regex.split(&rewards_text).collect();
            let amounts: Vec<String> = amount_regex
                .find_iter(&rewards_text)
                .map(|m| m.as_str().replace('×', "").trim().to_string())
                .collect();

            let mut rewards = Vec::new();
            for (i, reward) in reward_parts.iter().enumerate() {
                if i < amounts.len() {
                    let reward = reward.trim();
                    if !reward.is_empty() {
                        rewards.push(format!("{} x{}", reward, amounts[i]));
                    }
                }
            }

            if !rewards.is_empty() {
                codes.push(GameCode {
                    id: None,
                    code,
                    active: true,
                    date: current_time.into(),
                    rewards,
                    source: "fandom".to_string(),
                });
            }
        }
    }

    if codes.is_empty() {
        warn!("[StarRail][Codes][Fandom] No codes found");
    } else {
        debug!("[StarRail][Codes][Fandom] Found {} codes", codes.len());
    }

    Ok(codes)
}
