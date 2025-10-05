use crate::{config::Settings, types::GameCode};
use anyhow::Context;
use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let client = Client::new();
    let url = "https://zzz.gg/codes";

    let response = client
        .get(url)
        .header("User-Agent", &config.server.user_agent)
        .send()
        .await
        .context("Failed to fetch ZZZ.GG page")?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);

    let codes = parse_codes_from_html(&document)?;

    if codes.is_empty() {
        warn!("[Zenless][Codes][ZZZ.GG] No codes found");
    } else {
        debug!("[Zenless][Codes][ZZZ.GG] Found {} codes", codes.len());
    }

    Ok(codes)
}

fn parse_codes_from_html(document: &Html) -> anyhow::Result<Vec<GameCode>> {
    let tr_selector = Selector::parse("tr.active").unwrap();
    let code_selector = Selector::parse("td.code").unwrap();
    let reward_selector = Selector::parse("li.reward").unwrap();
    let count_selector = Selector::parse(".count").unwrap();
    let name_selector = Selector::parse(".name").unwrap();
    let current_time = Utc::now();

    let mut codes = Vec::new();

    for row in document.select(&tr_selector) {
        // Extract code
        if let Some(code_element) = row.select(&code_selector).next() {
            let code = code_element.text().collect::<String>().trim().to_string();

            // Extract rewards
            let mut rewards = Vec::new();
            for reward_element in row.select(&reward_selector) {
                if let (Some(count_element), Some(name_element)) = (
                    reward_element.select(&count_selector).next(),
                    reward_element.select(&name_selector).next(),
                ) {
                    let count_str = count_element.text().collect::<String>().trim().to_string();
                    let name_str = name_element.text().collect::<String>().trim().to_string();
                    rewards.push(format!("{}x {}", count_str, name_str));
                }
            }

            // Only add if we have both code and rewards
            if !code.is_empty() && !rewards.is_empty() {
                codes.push(GameCode {
                    id: None,
                    code,
                    active: true,
                    date: current_time.into(),
                    rewards,
                    source: "zzz.gg".to_string(),
                });
            }
        }
    }

    Ok(codes)
}
