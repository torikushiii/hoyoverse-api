use crate::{types::GameCode, config::Settings};
use reqwest::Client;
use scraper::{Html, Selector};
use anyhow::Context;
use tracing::{debug, warn};
use chrono::Utc;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://www.dudcode.com/code/zenless-zone-zero-codes/";

    let response = client.get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch DudCode page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let codes = parse_codes_from_html(&document)?;

    if codes.is_empty() {
        warn!("[Zenless][Codes][DudCode] No codes found");
    } else {
        debug!("[Zenless][Codes][DudCode] Found {} codes", codes.len());
    }

    Ok(codes)
}

fn parse_codes_from_html(document: &Html) -> anyhow::Result<Vec<GameCode>> {
    let li_selector = Selector::parse("ul li").unwrap();
    let current_time = Utc::now();
    let mut codes = Vec::new();

    for li in document.select(&li_selector) {
        let text = li.text().collect::<String>().trim().to_string();

        // Skip if there's no hyphen (meaning no rewards)
        if !text.contains('–') {
            continue;
        }

        // Split the text into code and rewards
        let parts: Vec<&str> = text.split('–').collect();
        if parts.len() != 2 {
            continue;
        }

        // Convert code to uppercase and remove non-alphanumeric characters
        let code = parts[0]
            .trim()
            .replace(|c: char| !c.is_ascii_alphanumeric(), "")
            .to_uppercase();
        let rewards_text = parts[1].trim();

        // Parse rewards with special handling for numbers with commas
        let mut rewards = Vec::new();
        let mut current_reward = String::new();
        let mut in_number = false;

        for c in rewards_text.chars() {
            if c == ',' && !in_number {
                if !current_reward.trim().is_empty() {
                    rewards.push(current_reward.trim().replace("Ppolychromes", "Polychromes"));
                }
                current_reward.clear();
            } else {
                if c.is_ascii_digit() {
                    in_number = true;
                } else if c == 'x' || c == ' ' {
                    in_number = false;
                }
                current_reward.push(c);
            }
        }

        if !current_reward.trim().is_empty() {
            rewards.push(current_reward.trim().replace("Ppolychromes", "Polychromes")); // xd?
        }

        if code.is_empty() || rewards.is_empty() {
            continue;
        }

        codes.push(GameCode {
            id: None,
            code,
            active: true,
            date: current_time.into(),
            rewards,
            source: "dudcode".to_string(),
        });
    }

    Ok(codes)
}